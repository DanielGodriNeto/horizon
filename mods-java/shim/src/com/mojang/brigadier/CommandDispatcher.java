package com.mojang.brigadier;

import com.mojang.brigadier.builder.LiteralArgumentBuilder;
import java.util.LinkedHashMap;
import java.util.Map;

/** Shim stub of Brigadier's CommandDispatcher: the registered words and what they run. */
public class CommandDispatcher<S> {
    private final Map<String, Command<S>> commands = new LinkedHashMap<>();

    public void register(LiteralArgumentBuilder<S> command) {
        if (command.getCommand() != null) commands.put(command.getLiteral(), command.getCommand());
    }

    public Map<String, Command<S>> commands() { return commands; }
}
