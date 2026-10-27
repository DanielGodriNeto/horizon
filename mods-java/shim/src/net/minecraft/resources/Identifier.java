package net.minecraft.resources;

/** Stub shim of the 26.1 resource id (was ResourceLocation). */
public final class Identifier {
    private final String namespace;
    private final String path;

    private Identifier(String namespace, String path) {
        this.namespace = namespace;
        this.path = path;
    }

    public static Identifier fromNamespaceAndPath(String namespace, String path) {
        return new Identifier(namespace, path);
    }

    public String getNamespace() { return namespace; }

    public String getPath() { return path; }

    @Override
    public String toString() { return namespace + ":" + path; }
}
