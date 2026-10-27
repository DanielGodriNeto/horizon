package net.minecraft.commands;

import java.util.function.Supplier;
import net.minecraft.network.chat.Component;

/** Shim stub: lines a command sends are collected and go back to the game. */
public final class CommandSourceStack {
    private final StringBuilder messages = new StringBuilder();

    public void sendSuccess(Supplier<Component> message, boolean broadcastToAdmins) {
        sendSystemMessage(message.get());
    }

    public void sendFailure(Component message) { sendSystemMessage(message); }

    public void sendSystemMessage(Component message) {
        if (!messages.isEmpty()) messages.append('\n');
        messages.append(message.getString());
    }

    public String takeMessages() {
        String out = messages.toString();
        messages.setLength(0);
        return out;
    }
}
