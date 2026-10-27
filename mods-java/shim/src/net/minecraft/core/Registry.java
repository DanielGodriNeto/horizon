package net.minecraft.core;

import net.minecraft.resources.Identifier;

/** Stub shim: registering records the entry for the host instead of touching a game registry. */
public final class Registry<T> {
    public final String name;

    public Registry(String name) { this.name = name; }

    public static <V, T extends V> T register(Registry<V> registry, Identifier id, T entry) {
        horizon.ShimHost.record(registry.name, id, entry);
        return entry;
    }
}
