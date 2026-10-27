import java.io.BufferedInputStream;
import java.io.BufferedOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;

/** Linha de base do bench P3: ecoa quadros (u32 LE de tamanho + payload) de stdin para stdout. */
public final class PipeEcho {
    private static final int MAX = 1 << 16;

    public static void main(String[] args) throws IOException {
        InputStream in = new BufferedInputStream(System.in, MAX);
        OutputStream out = new BufferedOutputStream(System.out, MAX);
        byte[] head = new byte[4];
        byte[] body = new byte[MAX];
        while (true) {
            if (in.available() == 0) {
                out.flush(); // fim de lote: tudo o que foi lido já foi respondido
            }
            if (!readFully(in, head, 4)) {
                break; // EOF
            }
            int len = (head[0] & 0xFF) | ((head[1] & 0xFF) << 8) | ((head[2] & 0xFF) << 16) | ((head[3] & 0xFF) << 24);
            if (len < 0 || len > MAX) {
                throw new IOException("quadro inválido: " + len);
            }
            if (!readFully(in, body, len)) {
                throw new IOException("EOF no meio do quadro");
            }
            out.write(head, 0, 4);
            out.write(body, 0, len);
        }
        out.flush();
    }

    /** Lê exatamente `len` bytes. Devolve false só se o EOF vier antes do primeiro byte. */
    private static boolean readFully(InputStream in, byte[] buf, int len) throws IOException {
        int done = 0;
        while (done < len) {
            int n = in.read(buf, done, len - done);
            if (n < 0) {
                if (done == 0) {
                    return false;
                }
                throw new IOException("EOF no meio da leitura");
            }
            done += n;
        }
        return true;
    }
}
