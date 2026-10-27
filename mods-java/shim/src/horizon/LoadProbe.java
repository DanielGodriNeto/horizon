package horizon;

import java.net.URL;
import java.net.URLClassLoader;
import java.nio.charset.StandardCharsets;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;
import java.util.regex.Matcher;
import java.util.regex.Pattern;
import java.util.zip.ZipEntry;
import java.util.zip.ZipFile;

/**
 * Coverage probe: loads a Fabric jar's server-side entrypoints ("main", "server") against the shim and prints
 * one line per entrypoint: {@code PROBE <entry-kind> <class> <outcome> <detail>}. Mixins are never applied, and a
 * jar that declares them is reported as reduced. Meant to run inside the sandbox (mods-java/sandbox).
 */
public final class LoadProbe {
    private static final String BS = Pattern.quote(String.valueOf((char) 92));
    private static final Pattern TOKEN = Pattern.compile("\"((?:[^\"" + BS + "]|" + BS + ".)*)\"( *:)?");
    private static final long TIMEOUT_MS = 10_000;

    private LoadProbe() {}

    public static void main(String[] args) throws Exception {
        Path jar = Path.of(args[0]);
        String manifest;
        try (ZipFile zip = new ZipFile(jar.toFile())) {
            ZipEntry e = zip.getEntry("fabric.mod.json");
            if (e == null) {
                System.out.println("PROBE - - NO_MANIFEST -");
                return;
            }
            manifest = new String(zip.getInputStream(e).readAllBytes(), StandardCharsets.UTF_8);
        }
        System.out.println("PROBE mixins - " + (manifest.contains("\"mixins\"") ? "REDUCED" : "NONE") + " -");
        URLClassLoader loader = new URLClassLoader(new URL[] {jar.toUri().toURL()}, LoadProbe.class.getClassLoader());
        boolean any = false;
        for (String kind : new String[] {"main", "server"}) {
            for (String[] entry : entries(manifest, kind)) {
                any = true;
                probe(loader, kind, entry[0], entry[1]);
            }
        }
        if (!any) System.out.println("PROBE - - NO_ENTRYPOINT -");
        System.exit(0);
    }

    /** (class, adapter) pairs of an entrypoint kind; tolerant regex reading, the shim has no JSON parser. */
    private static List<String[]> entries(String manifest, String kind) {
        List<String[]> out = new ArrayList<>();
        int at = manifest.indexOf("\"entrypoints\"");
        if (at < 0) return out;
        int key = manifest.indexOf("\"" + kind + "\"", at);
        int open = key < 0 ? -1 : manifest.indexOf('[', key);
        if (open < 0) return out;
        int close = manifest.indexOf(']', open);
        Matcher m = TOKEN.matcher(manifest.substring(open + 1, close));
        String lastKey = null;
        String[] current = null;
        while (m.find()) {
            if (m.group(2) != null) {
                lastKey = m.group(1);
                continue;
            }
            if (lastKey == null) {
                out.add(new String[] {m.group(1), "default"});
            } else if (lastKey.equals("value")) {
                current = new String[] {m.group(1), "default"};
                out.add(current);
            } else if (lastKey.equals("adapter") && current != null) {
                current[1] = m.group(1);
            }
            lastKey = null;
        }
        return out;
    }

    private static void probe(ClassLoader loader, String kind, String name, String adapter) throws Exception {
        if (!adapter.equals("default")) {
            System.out.println("PROBE " + kind + " " + name + " ADAPTER " + adapter);
            return;
        }
        String[] result = {"LOADED", "-"};
        Thread t = new Thread(() -> run(loader, name, kind, result));
        t.setDaemon(true);
        t.start();
        t.join(TIMEOUT_MS);
        String outcome = t.isAlive() ? "TIMEOUT" : result[0];
        System.out.println("PROBE " + kind + " " + name + " " + outcome + " " + result[1]);
    }

    private static void run(ClassLoader loader, String name, String kind, String[] result) {
        String stage = "load";
        try {
            Class<?> c = Class.forName(name, true, loader);
            stage = "new";
            Object entry = c.getDeclaredConstructor().newInstance();
            stage = "init";
            String method = kind.equals("main") ? "onInitialize" : "onInitializeServer";
            c.getMethod(method).invoke(entry);
        } catch (java.lang.reflect.InvocationTargetException e) {
            fail(result, stage, e.getCause());
        } catch (Throwable e) {
            fail(result, stage, e);
        }
    }

    private static void fail(String[] result, String stage, Throwable e) {
        String type = e.getClass().getSimpleName();
        String outcome = switch (type) {
            case "NoClassDefFoundError", "ClassNotFoundException" -> "MISSING_CLASS";
            case "NoSuchMethodError", "NoSuchFieldError", "AbstractMethodError", "IncompatibleClassChangeError",
                    "NoSuchMethodException", "NoSuchFieldException", "IllegalAccessError" -> "MISSING_MEMBER";
            default -> "ERROR";
        };
        String detail = type + "@" + stage + ":" + String.valueOf(e.getMessage()).replace('\n', ' ');
        result[0] = outcome;
        result[1] = detail.length() > 300 ? detail.substring(0, 300) : detail;
    }
}
