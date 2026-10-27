package net.fabricmc.fabric.api.event;

import java.util.ArrayList;
import java.util.List;

/** Shim stub of Fabric API's Event: listeners in registration order. */
public final class Event<T> {
    private final List<T> listeners = new ArrayList<>();

    public void register(T listener) { listeners.add(listener); }

    public List<T> listeners() { return listeners; }
}
