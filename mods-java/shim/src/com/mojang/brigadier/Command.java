package com.mojang.brigadier;

import com.mojang.brigadier.context.CommandContext;

/** Shim stub of Brigadier's Command. */
@FunctionalInterface
public interface Command<S> {
    int SINGLE_SUCCESS = 1;

    int run(CommandContext<S> context);
}
