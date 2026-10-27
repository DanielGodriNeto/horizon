package dev.horizon.gemjava;

import net.fabricmc.api.ModInitializer;
import net.minecraft.core.Registry;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.resources.Identifier;
import net.minecraft.world.item.Item;

/** Our own test mod: one item, written the way a Fabric 26.1 mod writes it. */
public final class GemMod implements ModInitializer {
    @Override
    public void onInitialize() {
        Registry.register(BuiltInRegistries.ITEM, Identifier.fromNamespaceAndPath("gemjava", "gem"),
                new Item(new Item.Properties().stacksTo(16)));
    }
}
