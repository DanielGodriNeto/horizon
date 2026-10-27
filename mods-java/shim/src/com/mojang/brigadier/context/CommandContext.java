package com.mojang.brigadier.context;

/** Shim stub of Brigadier's CommandContext: only the source (no arguments). */
public class CommandContext<S> {
    private final S source;

    public CommandContext(S source) { this.source = source; }

    public S getSource() { return source; }
}
