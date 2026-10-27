import java.io.IOException;
import java.net.InetSocketAddress;
import java.net.Socket;
import java.nio.file.Files;
import java.nio.file.Path;

/** Sandbox proof: every line printed OK must be FAIL except the write inside the run dir. */
public final class ProofProbe {
    public static void main(String[] args) {
        String outside = args[0];
        check("net-internet", () -> { try (Socket s = new Socket()) { s.connect(new InetSocketAddress("1.1.1.1", 80), 3000); } });
        check("net-loopback", () -> { try (Socket s = new Socket()) { s.connect(new InetSocketAddress("127.0.0.1", 135), 3000); } });
        check("read-user-file", () -> Files.readAllBytes(Path.of(outside, "secret.txt")));
        check("write-outside", () -> Files.writeString(Path.of(outside, "pwned.txt"), "x"));
        check("spawn-process", () -> new ProcessBuilder("cmd", "/c", "echo hi").start().waitFor());
        check("write-rundir", () -> Files.writeString(Path.of("proof-write.txt"), "ok"));
    }

    interface Action { void run() throws Exception; }

    private static void check(String what, Action a) {
        try {
            a.run();
            System.out.println("PROOF " + what + " OK");
        } catch (Throwable e) {
            System.out.println("PROOF " + what + " FAIL " + e.getClass().getSimpleName() + " " + e.getMessage() + (e.getCause() == null ? "" : " <- " + e.getCause()));
        }
    }
}
