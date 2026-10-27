package net.minecraft.server.players;

import horizon.Outbox;
import net.minecraft.network.chat.Component;

/** Shim stub: a broadcast goes to the game as a line to every player. */
public final class PlayerList {
    public void broadcastSystemMessage(Component message, boolean overlay) { Outbox.add(message.getString()); }
}
