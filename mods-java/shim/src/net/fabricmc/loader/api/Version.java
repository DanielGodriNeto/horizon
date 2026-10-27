package net.fabricmc.loader.api;

public interface Version extends Comparable<Version> {
    String getFriendlyString();
}
