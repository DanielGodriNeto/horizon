package net.minecraft.commands;

import com.mojang.brigadier.builder.LiteralArgumentBuilder;

/** Shim stub of Commands: the literal builder and the selection enum. */
public final class Commands {
    private Commands() {}

    public static LiteralArgumentBuilder<CommandSourceStack> literal(String name) {
        return LiteralArgumentBuilder.literal(name);
    }

    public enum CommandSelection { ALL, DEDICATED, INTEGRATED }
}
