package net.fabricmc.fabric.api.event.lifecycle.v1;

import net.fabricmc.fabric.api.event.Event;
import net.minecraft.server.MinecraftServer;

/** Shim stub of Fabric API's ServerTickEvents (only END_SERVER_TICK). */
public final class ServerTickEvents {
    public static final Event<EndTick> END_SERVER_TICK = new Event<>();

    private ServerTickEvents() {}

    @FunctionalInterface
    public interface EndTick {
        void onEndTick(MinecraftServer server);
    }
}
