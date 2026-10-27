package com.mojang.brigadier.builder;

import com.mojang.brigadier.Command;

/** Shim stub of Brigadier's LiteralArgumentBuilder: a word and what it runs (no arguments or children). */
public class LiteralArgumentBuilder<S> {
    private final String literal;
    private Command<S> command;

    protected LiteralArgumentBuilder(String literal) { this.literal = literal; }

    public static <S> LiteralArgumentBuilder<S> literal(String name) { return new LiteralArgumentBuilder<>(name); }

    public LiteralArgumentBuilder<S> executes(Command<S> command) {
        this.command = command;
        return this;
    }

    public String getLiteral() { return literal; }

    public Command<S> getCommand() { return command; }
}
