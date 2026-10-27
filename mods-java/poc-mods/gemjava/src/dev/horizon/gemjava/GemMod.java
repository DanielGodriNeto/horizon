package dev.horizon.gemjava;

import net.fabricmc.api.ModInitializer;
import net.fabricmc.fabric.api.event.player.PlayerBlockBreakEvents;
import net.minecraft.core.Registry;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.resources.Identifier;
import net.minecraft.world.item.Item;

/** Our own test mod: one item and a block-break callback, written the way a Fabric 26.1 mod writes them. */
public final class GemMod implements ModInitializer {
    private int breaks;

    @Override
    public void onInitialize() {
        Registry.register(BuiltInRegistries.ITEM, Identifier.fromNamespaceAndPath("gemjava", "gem"),
                new Item(new Item.Properties().stacksTo(16)));
        PlayerBlockBreakEvents.AFTER.register((world, player, pos, state, blockEntity) -> {
            breaks++;
            player.sendSystemMessage("gem " + breaks + " " + state.blockId() + " " + pos.getY());
        });
    }
}
