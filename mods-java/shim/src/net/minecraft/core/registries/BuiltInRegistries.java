package net.minecraft.core.registries;

import net.minecraft.core.Registry;
import net.minecraft.world.item.Item;

public final class BuiltInRegistries {
    public static final Registry<Item> ITEM = new Registry<>("item");

    private BuiltInRegistries() {}
}
