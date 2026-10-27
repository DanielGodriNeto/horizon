package dev.horizon.gemjava;

import com.mojang.brigadier.Command;
import net.fabricmc.api.ModInitializer;
import net.fabricmc.fabric.api.command.v2.CommandRegistrationCallback;
import net.fabricmc.fabric.api.event.lifecycle.v1.ServerTickEvents;
import net.fabricmc.fabric.api.event.player.PlayerBlockBreakEvents;
import net.minecraft.commands.Commands;
import net.minecraft.core.Registry;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.network.chat.Component;
import net.minecraft.resources.Identifier;
import net.minecraft.world.item.Item;

/** Our own test mod: an item, a block-break callback, a tick callback and a command, written the way a Fabric 26.1 mod writes them. */
public final class GemMod implements ModInitializer {
    private int breaks;
    private int ticks;

    @Override
    public void onInitialize() {
        Registry.register(BuiltInRegistries.ITEM, Identifier.fromNamespaceAndPath("gemjava", "gem"),
                new Item(new Item.Properties().stacksTo(16)));
        PlayerBlockBreakEvents.AFTER.register((world, player, pos, state, blockEntity) -> {
            breaks++;
            player.sendSystemMessage("gem " + breaks + " " + state.blockId() + " " + pos.getY());
        });
        ServerTickEvents.END_SERVER_TICK.register(server -> {
            ticks++;
            if (ticks % 200 == 0) {
                server.getPlayerList().broadcastSystemMessage(Component.literal("gem tick " + ticks), false);
            }
        });
        CommandRegistrationCallback.EVENT.register((dispatcher, registryAccess, environment) ->
                dispatcher.register(Commands.literal("gemcount").executes(ctx -> {
                    ctx.getSource().sendSuccess(() -> Component.literal("gem breaks=" + breaks + " ticks=" + ticks), false);
                    return Command.SINGLE_SUCCESS;
                })));
    }
}
