package net.minecraft.world.item;

/** Stub shim: only the properties the host can turn into a content item. */
public class Item {
    public final Properties properties;

    public Item(Properties properties) { this.properties = properties; }

    public static class Properties {
        public int stackSize = 64;

        public Properties stacksTo(int size) {
            this.stackSize = size;
            return this;
        }
    }
}
