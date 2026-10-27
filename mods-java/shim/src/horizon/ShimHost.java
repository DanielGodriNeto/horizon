package horizon;

import java.util.ArrayList;
import java.util.List;
import net.minecraft.resources.Identifier;
import net.minecraft.world.item.Item;

/** Collects what a mod registers while its entrypoints run. */
public final class ShimHost {
    public record Entry(String registry, Identifier id, Object value) {}

    private static final List<Entry> ENTRIES = new ArrayList<>();

    private ShimHost() {}

    public static void record(String registry, Identifier id, Object value) {
        ENTRIES.add(new Entry(registry, id, value));
    }

    public static List<Entry> entries() { return ENTRIES; }

    /** The horizon content file for an item entry: template diamond, the mod's stack size. */
    public static String itemJson(Item item) {
        return "{\"template\": \"minecraft:diamond\", \"max_stack_size\": " + item.properties.stackSize + "}";
    }
}
