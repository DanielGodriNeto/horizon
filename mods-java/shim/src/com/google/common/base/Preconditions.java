package com.google.common.base;

/** The checks mods use at startup; messages are the plain ones. */
public final class Preconditions {
    private Preconditions() {}
    public static <T> T checkNotNull(T ref) { if (ref == null) throw new NullPointerException(); return ref; }
    public static <T> T checkNotNull(T ref, Object message) { if (ref == null) throw new NullPointerException(String.valueOf(message)); return ref; }
    public static void checkArgument(boolean ok) { if (!ok) throw new IllegalArgumentException(); }
    public static void checkArgument(boolean ok, Object message) { if (!ok) throw new IllegalArgumentException(String.valueOf(message)); }
    public static void checkState(boolean ok) { if (!ok) throw new IllegalStateException(); }
    public static void checkState(boolean ok, Object message) { if (!ok) throw new IllegalStateException(String.valueOf(message)); }
}
