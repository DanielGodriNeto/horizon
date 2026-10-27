package net.minecraft.world.entity.player;

/** Shim stub: a message sent to the player is collected and goes back to the game. */
public class Player {
    private final StringBuilder messages = new StringBuilder();

    public void sendSystemMessage(String text) {
        if (!messages.isEmpty()) messages.append('\n');
        messages.append(text);
    }

    public String takeMessages() {
        String out = messages.toString();
        messages.setLength(0);
        return out;
    }
}
