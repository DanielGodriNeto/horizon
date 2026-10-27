package horizon;

import java.io.IOException;
import java.io.OutputStream;
import java.net.URL;
import java.net.URLClassLoader;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;
import java.util.regex.Matcher;
import java.util.regex.Pattern;
import java.util.zip.ZipEntry;
import java.util.zip.ZipFile;
import java.util.zip.ZipOutputStream;
import net.fabricmc.api.ModInitializer;
import net.minecraft.world.item.Item;

/**
 * Runs a Fabric mod's main entrypoints against the shim and writes what it registered as a
 * data-only jar the game's content loader reads: {@code ModHost <mod.jar> <content.jar>}.
 */
public final class ModHost {
    private static final Pattern ID = Pattern.compile("\"id\"\s*:\s*\"([a-z0-9_]+)\"");
    private static final Pattern MAIN = Pattern.compile("\"main\"\\s*:\\s*\\[([^\\]]*)\\]");
    private static final Pattern STR = Pattern.compile("\"([^\"]+)\"");

    private ModHost() {}

    public static void main(String[] args) throws Exception {
        if (args.length != 2) {
            System.err.println("usage: ModHost <mod.jar> <content.jar>");
            System.exit(2);
        }
        String id = load(Path.of(args[0]));
        write(Path.of(args[1]), id);
        System.out.println("registered " + ShimHost.entries().size() + " entries from " + id);
    }

    /** Runs the main entrypoints and returns the mod id. The class loader stays open: the mod's callbacks load classes later. */
    static String load(Path mod) throws Exception {
        String manifest = readManifest(mod);
        String id = first(ID, manifest, "id");
        URLClassLoader loader = new URLClassLoader(new URL[] {mod.toUri().toURL()}, ModHost.class.getClassLoader());
        for (String name : entrypoints(manifest)) {
            Object entry = loader.loadClass(name).getDeclaredConstructor().newInstance();
            ((ModInitializer) entry).onInitialize();
        }
        return id;
    }

    private static String readManifest(Path jar) throws IOException {
        try (ZipFile zip = new ZipFile(jar.toFile())) {
            ZipEntry e = zip.getEntry("fabric.mod.json");
            if (e == null) throw new IOException("no fabric.mod.json");
            return new String(zip.getInputStream(e).readAllBytes(), StandardCharsets.UTF_8);
        }
    }

    private static String first(Pattern p, String text, String what) {
        Matcher m = p.matcher(text);
        if (!m.find()) throw new IllegalArgumentException("manifest has no " + what);
        return m.group(1);
    }

    private static List<String> entrypoints(String manifest) {
        List<String> out = new ArrayList<>();
        Matcher main = MAIN.matcher(manifest);
        if (main.find()) {
            Matcher s = STR.matcher(main.group(1));
            while (s.find()) out.add(s.group(1));
        }
        return out;
    }

    private static void write(Path out, String id) throws IOException {
        try (OutputStream os = Files.newOutputStream(out); ZipOutputStream zip = new ZipOutputStream(os)) {
            put(zip, "fabric.mod.json", "{\"schemaVersion\":1,\"id\":\"" + id
                    + "\",\"version\":\"1.0.0\",\"name\":\"" + id + " (shim)\",\"environment\":\"*\"}");
            for (ShimHost.Entry e : ShimHost.entries()) {
                if (e.registry().equals("item") && e.value() instanceof Item item) {
                    put(zip, "data/" + e.id().getNamespace() + "/horizon/item/" + e.id().getPath() + ".json",
                            ShimHost.itemJson(item));
                }
            }
        }
    }

    private static void put(ZipOutputStream zip, String name, String text) throws IOException {
        zip.putNextEntry(new ZipEntry(name));
        zip.write(text.getBytes(StandardCharsets.UTF_8));
        zip.closeEntry();
    }
}
