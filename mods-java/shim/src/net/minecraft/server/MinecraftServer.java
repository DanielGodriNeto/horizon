package net.minecraft.server;

import net.minecraft.server.players.PlayerList;

/** Shim stub: the server a tick callback gets. */
public final class MinecraftServer {
    private final PlayerList players = new PlayerList();
    private int tickCount;

    public PlayerList getPlayerList() { return players; }

    public int getTickCount() { return tickCount; }

    public void setTickCount(int tickCount) { this.tickCount = tickCount; }
}
