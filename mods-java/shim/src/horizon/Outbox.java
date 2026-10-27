package horizon;

import java.util.ArrayList;
import java.util.List;

/** Lines a mod sent outside a player or command context (a tick broadcast); the host sends them back after the callback. */
public final class Outbox {
    private static final List<String> LINES = new ArrayList<>();

    private Outbox() {}

    public static void add(String line) { LINES.add(line); }

    public static String take() {
        String out = String.join("\n", LINES);
        LINES.clear();
        return out;
    }
}
