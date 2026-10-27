import java.io.IOException;
import java.io.InputStream;
import java.io.UncheckedIOException;
import java.lang.foreign.Arena;
import java.lang.foreign.MemorySegment;
import java.lang.foreign.ValueLayout;
import java.lang.invoke.VarHandle;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.nio.channels.FileChannel;
import java.nio.file.Path;
import java.nio.file.StandardOpenOption;

/** Sidecar P3 (lado Java, modo echo). Protocolo em mods-java/PROTOCOLO.md, adendo v1.1. */
public final class Sidecar {
    static final int MAGIC = 0x484D4A31;
    static final int VERSION = 1;
    static final int HEADER = 64;
    static final int CTRL = 128;
    static final int HEADER_READ = 12;
    static final long OFF_CAP = 8L;
    static final long OFF_READY = 16L;
    static final long OFF_SHUTDOWN = 20L;
    static final long OFF_SLEEPING = 24L;
    static final long HEAD_OFF = 0L;
    static final long TAIL_OFF = 64L;
    static final long DATA_OFF = 128L;
    static final int SKIP = 0xFFFFFFFF;
    static final int REC_HDR = 4;
    static final long SPIN_NANOS = 50_000L;
    static final long NOT_IDLE = Long.MIN_VALUE;

    /** Acesso por offset em bytes (coordenadas: MemorySegment, long). Campos são alinhados. */
    static final VarHandle INT_VH = ValueLayout.JAVA_INT.withOrder(ByteOrder.LITTLE_ENDIAN).varHandle();
    static final VarHandle LONG_VH = ValueLayout.JAVA_LONG.withOrder(ByteOrder.LITTLE_ENDIAN).varHandle();

    static int getInt(MemorySegment seg, long off) {
        return (int) INT_VH.get(seg, off);
    }

    static void setInt(MemorySegment seg, long off, int value) {
        INT_VH.set(seg, off, value);
    }

    static int loadIntVolatile(MemorySegment seg, long off) {
        return (int) INT_VH.getVolatile(seg, off);
    }

    static void storeIntVolatile(MemorySegment seg, long off, int value) {
        INT_VH.setVolatile(seg, off, value);
    }

    static long loadLongVolatile(MemorySegment seg, long off) {
        return (long) LONG_VH.getVolatile(seg, off);
    }

    static long loadLongAcquire(MemorySegment seg, long off) {
        return (long) LONG_VH.getAcquire(seg, off);
    }

    static void storeLongRelease(MemorySegment seg, long off, long value) {
        LONG_VH.setRelease(seg, off, value);
    }

    /** Um anel: controle de 128 bytes em {@code base}, dados em {@code base + 128}. */
    record Ring(MemorySegment seg, long base) {
        long loadHead() {
            return loadLongAcquire(seg, base + HEAD_OFF);
        }

        long loadTail() {
            return loadLongAcquire(seg, base + TAIL_OFF);
        }

        void storeHead(long head) {
            storeLongRelease(seg, base + HEAD_OFF, head);
        }

        void storeTail(long tail) {
            storeLongRelease(seg, base + TAIL_OFF, tail);
        }

        long dataAt(long pos) {
            return base + DATA_OFF + pos;
        }
    }

    /** Laço de echo: anel A (entrada) para anel B (saída), com despertar por stdin. */
    static final class Relay {
        private final MemorySegment seg;
        private final InputStream wake;
        private final long cap;
        private final Ring in;
        private final Ring out;
        private long inHead;
        private long outTail;
        private long idleSince = NOT_IDLE;
        private boolean eof;

        Relay(MemorySegment seg, InputStream wake) {
            this.seg = seg;
            this.wake = wake;
            this.cap = getInt(seg, OFF_CAP) & 0xFFFFFFFFL;
            this.in = new Ring(seg, HEADER);
            this.out = new Ring(seg, HEADER + CTRL + cap);
            this.inHead = in.loadHead();
            this.outTail = out.loadTail();
        }

        void loop() {
            while (true) {
                boolean stop = eof || shutdownRequested();
                if (drainOnce()) {
                    idleSince = NOT_IDLE;
                } else if (stop) {
                    return;
                } else {
                    idle();
                }
            }
        }

        /** Copia tudo o que já foi publicado em A; publica o tail de B uma vez. */
        private boolean drainOnce() {
            long tailIn = in.loadTail();
            if (tailIn == inHead) {
                return false;
            }
            while (inHead < tailIn) {
                inHead = relayOne(tailIn);
                in.storeHead(inHead);
            }
            out.storeTail(outTail);
            return true;
        }

        private long relayOne(long tailIn) {
            long pos = inHead % cap;
            int len = getInt(seg, in.dataAt(pos));
            if (len == SKIP) {
                return inHead + (cap - pos);
            }
            check(len >= 0 && len <= cap - REC_HDR, "tamanho de registro invalido: " + len);
            long rec = recordSize(len);
            check(pos + rec <= cap && inHead + rec <= tailIn, "registro fora do anel publicado");
            forward(in.dataAt(pos), rec);
            return inHead + rec;
        }

        /** Copia um registro para B; se não cabe até o fim, grava o marcador de pulo e vai para 0. */
        private void forward(long src, long rec) {
            long pos = outTail % cap;
            long pad = pos + rec > cap ? cap - pos : 0;
            awaitFree(pad + rec);
            if (pad > 0) {
                setInt(seg, out.dataAt(pos), SKIP);
                outTail += pad;
                pos = 0;
            }
            MemorySegment.copy(seg, src, seg, out.dataAt(pos), rec);
            outTail += rec;
        }

        /** Espera espaço em B (o pai drena B; sem despertar por pipe aqui). Publica o tail antes de esperar. */
        private void awaitFree(long need) {
            check(need <= cap, "registro maior que o anel B");
            if (freeOut() >= need) {
                return;
            }
            out.storeTail(outTail);
            while (freeOut() < need) {
                Thread.yield();
            }
            idleSince = NOT_IDLE;
        }

        private long freeOut() {
            return cap - (outTail - out.loadHead());
        }

        private boolean shutdownRequested() {
            return loadIntVolatile(seg, OFF_SHUTDOWN) != 0;
        }

        /** Espera ocupada de SPIN_NANOS; depois dorme até o pai escrever um byte no stdin. */
        private void idle() {
            long now = System.nanoTime();
            if (idleSince == NOT_IDLE) {
                idleSince = now;
            }
            if (now - idleSince < SPIN_NANOS) {
                Thread.onSpinWait();
            } else {
                sleepUntilWake();
            }
        }

        /** Handshake do adendo v1.1: sleeping = 1, reconfere trabalho (volatile), bloqueia no stdin. */
        private void sleepUntilWake() {
            storeIntVolatile(seg, OFF_SLEEPING, 1);
            if (!workPending()) {
                readWakeByte();
            }
            storeIntVolatile(seg, OFF_SLEEPING, 0);
            idleSince = NOT_IDLE;
        }

        /** Tail relido com volatile: a escrita de sleeping e esta leitura precisam de ordem total. */
        private boolean workPending() {
            return loadLongVolatile(seg, in.base() + TAIL_OFF) != inHead || shutdownRequested();
        }

        /** Lê um byte (EOF = pai morreu: marca eof) e descarta o que mais chegou. */
        private void readWakeByte() {
            try {
                if (wake.read() < 0) {
                    eof = true;
                } else {
                    wake.skipNBytes(wake.available());
                }
            } catch (IOException e) {
                throw new UncheckedIOException(e);
            }
        }
    }

    public static void main(String[] args) {
        try {
            serve(args);
            System.out.println("bye");
        } catch (IllegalArgumentException e) {
            System.err.println("sidecar: " + e.getMessage());
            System.exit(2);
        } catch (Exception e) {
            System.err.println("sidecar: " + e);
            e.printStackTrace();
            System.exit(1);
        }
    }

    static void serve(String[] args) throws IOException {
        if (args.length != 2 || !"echo".equals(args[1])) {
            throw new IllegalArgumentException("uso: java -cp out Sidecar <arquivo-mapeado> echo");
        }
        try (FileChannel ch = FileChannel.open(Path.of(args[0]), StandardOpenOption.READ, StandardOpenOption.WRITE);
             Arena arena = Arena.ofShared()) {
            long cap = checkHeader(readHeader(ch));
            long size = HEADER + 2L * (CTRL + cap);
            if (ch.size() < size) {
                throw new IllegalArgumentException("arquivo com " + ch.size() + " bytes; cabecalho exige " + size);
            }
            MemorySegment seg = ch.map(FileChannel.MapMode.READ_WRITE, 0, size, arena);
            run(seg, System.in);
        }
    }

    static ByteBuffer readHeader(FileChannel ch) throws IOException {
        ByteBuffer h = ByteBuffer.allocate(HEADER_READ).order(ByteOrder.LITTLE_ENDIAN);
        if (ch.read(h, 0) < HEADER_READ) {
            throw new IllegalArgumentException("arquivo menor que o cabecalho");
        }
        return h;
    }

    static long checkHeader(ByteBuffer h) {
        int magic = h.getInt(0);
        if (magic != MAGIC) {
            throw new IllegalArgumentException(String.format("magic invalido: 0x%08X", magic));
        }
        int version = h.getInt(4);
        if (version != VERSION) {
            throw new IllegalArgumentException("versao nao suportada: " + version);
        }
        long cap = h.getInt(8) & 0xFFFFFFFFL;
        if (cap < 8 || (cap & (cap - 1)) != 0) {
            throw new IllegalArgumentException("CAP invalido (potencia de 2 >= 8): " + cap);
        }
        return cap;
    }

    /** Sinaliza pronto (java_ready = 1) e roda o laço de echo até shutdown ou EOF com A vazio. */
    static void run(MemorySegment seg, InputStream wake) {
        storeIntVolatile(seg, OFF_READY, 1);
        new Relay(seg, wake).loop();
    }

    /** Tamanho do registro: u32 len + payload, arredondado para múltiplo de 8. */
    static long recordSize(int len) {
        return (REC_HDR + len + 7L) & ~7L;
    }

    static void check(boolean ok, String msg) {
        if (!ok) {
            throw new IllegalStateException(msg);
        }
    }
}
