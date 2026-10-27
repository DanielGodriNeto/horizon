import java.io.IOException;
import java.io.OutputStream;
import java.io.PipedInputStream;
import java.io.PipedOutputStream;
import java.lang.foreign.Arena;
import java.lang.foreign.MemorySegment;
import java.lang.invoke.VarHandle;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.nio.channels.FileChannel;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.StandardOpenOption;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicReference;
import java.util.concurrent.locks.LockSupport;

/** Teste de ida e volta do anel: este teste faz o papel do Rust e confere cada resposta do Sidecar. */
public final class SelfTest {
    static final int CAP = 4096;
    static final int MESSAGES = 1000;
    static final int MAX_LEN = 300;
    static final int MAX_BATCH = 8;
    static final long DEADLINE_NANOS = TimeUnit.SECONDS.toNanos(30);
    static final long JOIN_MILLIS = 10_000L;
    static final long POLL_NANOS = 20_000L;
    static final long IDLE_MILLIS = 2L;
    static final long MAX_WAKE_NANOS = TimeUnit.MILLISECONDS.toNanos(5);

    public static void main(String[] args) throws IOException {
        Path file = Files.createTempFile("sidecar-selftest", ".bin");
        file.toFile().deleteOnExit();
        try {
            runTest(file);
            System.out.println("SELFTEST OK");
        } catch (Exception e) {
            System.err.println("SELFTEST FAIL: " + e);
            e.printStackTrace();
            System.exit(1);
        }
    }

    static void runTest(Path file) throws Exception {
        long size = Sidecar.HEADER + 2L * (Sidecar.CTRL + CAP);
        try (FileChannel ch = FileChannel.open(file, StandardOpenOption.READ, StandardOpenOption.WRITE);
             Arena arena = Arena.ofShared()) {
            writeHeader(ch, size);
            MemorySegment seg = ch.map(FileChannel.MapMode.READ_WRITE, 0, size, arena);
            AtomicReference<Throwable> failure = new AtomicReference<>();
            PipedInputStream stdin = new PipedInputStream();
            PipedOutputStream wake = new PipedOutputStream(stdin);
            Thread sidecar = new Thread(() -> runSidecar(seg, stdin, failure), "sidecar");
            sidecar.setDaemon(true);
            sidecar.start();
            awaitReady(seg, failure);
            Producer producer = new Producer(seg, wake, failure);
            awaitIdleSleep(seg, failure);
            long warmup = producer.echoAfterWake();
            awaitIdleSleep(seg, failure);
            long latency = producer.echoAfterWake();
            System.out.println("wake latency aquecimento: " + warmup / 1_000 + " us; medido: " + latency / 1_000 + " us");
            Sidecar.check(latency < MAX_WAKE_NANOS, "acordou e ecoou em " + latency / 1_000 + " us (limite 5000 us)");
            producer.exchange();
            Sidecar.storeIntVolatile(seg, Sidecar.OFF_SHUTDOWN, 1);
            sendWakeAfterShutdown(wake, sidecar);
            sidecar.join(JOIN_MILLIS);
            Sidecar.check(!sidecar.isAlive(), "sidecar nao saiu apos shutdown");
            Sidecar.check(failure.get() == null, "sidecar falhou: " + failure.get());
            System.out.println("wake latency: " + latency / 1_000 + " us");
        }
    }

    static void runSidecar(MemorySegment seg, PipedInputStream stdin, AtomicReference<Throwable> failure) {
        try {
            Sidecar.run(seg, stdin);
        } catch (Throwable t) {
            failure.set(t);
        }
    }

    static void awaitReady(MemorySegment seg, AtomicReference<Throwable> failure) {
        long deadline = System.nanoTime() + DEADLINE_NANOS;
        while (Sidecar.loadIntVolatile(seg, Sidecar.OFF_READY) != 1) {
            Sidecar.check(failure.get() == null, "sidecar falhou: " + failure.get());
            Sidecar.check(System.nanoTime() < deadline, "java_ready nao foi gravado");
            LockSupport.parkNanos(POLL_NANOS);
        }
    }

    /** Ocioso por mais de 1 ms: o sidecar tem de ter entrado em sleeping = 1. */
    static void awaitIdleSleep(MemorySegment seg, AtomicReference<Throwable> failure) throws InterruptedException {
        Thread.sleep(IDLE_MILLIS);
        long deadline = System.nanoTime() + DEADLINE_NANOS;
        while (Sidecar.loadIntVolatile(seg, Sidecar.OFF_SLEEPING) != 1) {
            Sidecar.check(failure.get() == null, "sidecar falhou: " + failure.get());
            Sidecar.check(System.nanoTime() < deadline, "sidecar nao entrou em sleeping = 1");
            Thread.onSpinWait();
        }
    }

    /** Depois do shutdown, o adendo manda acordar com 1 byte. Se o sidecar já saiu, o pipe recusa: ok. */
    static void sendWakeAfterShutdown(OutputStream wake, Thread sidecar) throws IOException {
        try {
            wake.write(1);
            wake.flush();
        } catch (IOException e) {
            Sidecar.check(!sidecar.isAlive(), "falha ao acordar o sidecar: " + e);
        }
    }

    static void writeHeader(FileChannel ch, long size) throws IOException {
        ByteBuffer h = ByteBuffer.allocate(Sidecar.HEADER_READ).order(ByteOrder.LITTLE_ENDIAN)
                .putInt(Sidecar.MAGIC).putInt(Sidecar.VERSION).putInt(CAP);
        h.flip();
        ch.write(h, 0);
        ch.write(ByteBuffer.allocate(1), size - 1);
    }

    static int lengthOf(int i) {
        return (i * 97) % (MAX_LEN + 1);
    }

    static byte[] payload(int i) {
        byte[] p = new byte[lengthOf(i)];
        for (int j = 0; j < p.length; j++) {
            p[j] = (byte) (i >>> ((j & 3) * 8));
        }
        return p;
    }

    /** Papel do Rust: escreve no anel A, acorda o sidecar pelo pipe se sleeping = 1, lê o anel B e confere. */
    static final class Producer {
        private final MemorySegment seg;
        private final OutputStream wake;
        private final AtomicReference<Throwable> failure;
        private final Sidecar.Ring a;
        private final Sidecar.Ring b;
        private long tailA;
        private long headB;
        private int sent;
        private int received;
        private int skips;
        private int wakes;

        Producer(MemorySegment seg, OutputStream wake, AtomicReference<Throwable> failure) {
            this.seg = seg;
            this.wake = wake;
            this.failure = failure;
            this.a = new Sidecar.Ring(seg, Sidecar.HEADER);
            this.b = new Sidecar.Ring(seg, Sidecar.HEADER + Sidecar.CTRL + CAP);
        }

        /** Uma mensagem, do envio até o eco; devolve o tempo em ns. Exige que o pipe tenha acordado o sidecar. */
        long echoAfterWake() throws IOException {
            int wakesBefore = wakes;
            long t0 = System.nanoTime();
            writeBatch(1);
            long deadline = t0 + DEADLINE_NANOS;
            while (readResponses() == 0) {
                Sidecar.check(System.nanoTime() < deadline, "timeout: sem eco apos wake");
                Sidecar.check(failure.get() == null, "sidecar falhou: " + failure.get());
                Thread.onSpinWait();
            }
            long latency = System.nanoTime() - t0;
            Sidecar.check(wakes == wakesBefore + 1, "o pipe nao acordou o sidecar (wakes=" + wakes + ")");
            return latency;
        }

        void exchange() throws IOException {
            long deadline = System.nanoTime() + DEADLINE_NANOS;
            while (received < MESSAGES) {
                Sidecar.check(System.nanoTime() < deadline,
                        "timeout: enviados=" + sent + " recebidos=" + received);
                Sidecar.check(failure.get() == null, "sidecar falhou: " + failure.get());
                int wrote = writeBatch(MAX_BATCH);
                int got = readResponses();
                if (wrote == 0 && got == 0) {
                    LockSupport.parkNanos(POLL_NANOS);
                }
            }
            Sidecar.check(b.loadTail() == headB, "sobrou dado no anel B");
            Sidecar.check(skips > 0, "marcador de pulo nao foi exercitado");
            Sidecar.check(tailA >= 4L * CAP, "anel nao deu a volta varias vezes: " + tailA);
        }

        private int writeBatch(int limit) throws IOException {
            int n = 0;
            while (n < limit && sent < MESSAGES) {
                byte[] p = payload(sent);
                if (!fits(Sidecar.recordSize(p.length))) {
                    break;
                }
                writeRecord(p);
                sent++;
                n++;
            }
            if (n > 0) {
                a.storeTail(tailA);
                wakeIfSleeping();
            }
            return n;
        }

        /** Adendo v1.1: fence cheio depois de publicar tail, e 1 byte no pipe se o sidecar dorme. */
        private void wakeIfSleeping() throws IOException {
            VarHandle.fullFence();
            if (Sidecar.loadIntVolatile(seg, Sidecar.OFF_SLEEPING) == 1) {
                wake.write(1);
                wake.flush();
                wakes++;
            }
        }

        private boolean fits(long rec) {
            long pos = tailA % CAP;
            long need = (pos + rec > CAP ? CAP - pos : 0) + rec;
            return CAP - (tailA - a.loadHead()) >= need;
        }

        private void writeRecord(byte[] p) {
            long rec = Sidecar.recordSize(p.length);
            long pos = tailA % CAP;
            if (pos + rec > CAP) {
                Sidecar.setInt(seg, a.dataAt(pos), Sidecar.SKIP);
                tailA += CAP - pos;
                skips++;
                pos = 0;
            }
            long off = a.dataAt(pos);
            seg.asSlice(off, rec).fill((byte) 0);
            Sidecar.setInt(seg, off, p.length);
            MemorySegment.copy(MemorySegment.ofArray(p), 0, seg, off + Sidecar.REC_HDR, p.length);
            tailA += rec;
        }

        private int readResponses() {
            long tailB = b.loadTail();
            int before = received;
            while (headB < tailB) {
                long pos = headB % CAP;
                long off = b.dataAt(pos);
                int len = Sidecar.getInt(seg, off);
                if (len == Sidecar.SKIP) {
                    headB += CAP - pos;
                } else {
                    verify(off, len);
                    headB += Sidecar.recordSize(len);
                    received++;
                }
                b.storeHead(headB);
            }
            return received - before;
        }

        private void verify(long off, int len) {
            Sidecar.check(received < sent, "resposta sem envio: " + received);
            byte[] expected = payload(received);
            Sidecar.check(len == expected.length,
                    "tamanho diferente na resposta " + received + ": " + len + " != " + expected.length);
            long diff = seg.asSlice(off + Sidecar.REC_HDR, len).mismatch(MemorySegment.ofArray(expected));
            Sidecar.check(diff == -1, "conteudo diferente na resposta " + received + " no byte " + diff);
        }
    }
}
