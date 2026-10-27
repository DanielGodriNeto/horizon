package net.fabricmc.loader.api;

import java.nio.file.Path;
import java.util.Collection;
import java.util.List;
import java.util.Optional;
import net.fabricmc.api.EnvType;

/** Shim loader: a dedicated server with no other mods; paths are the working directory. */
public interface FabricLoader {
    FabricLoader INSTANCE = new FabricLoader() {};

    static FabricLoader getInstance() { return INSTANCE; }

    default boolean isModLoaded(String id) { return false; }
    /** Every id resolves to a bare container: a mod asks for its own before anything else is loaded. */
    default Optional<ModContainer> getModContainer(String id) {
        Version version = new Version() {
            public String getFriendlyString() { return "0"; }
            public int compareTo(Version o) { return 0; }
        };
        net.fabricmc.loader.api.metadata.ModMetadata meta = new net.fabricmc.loader.api.metadata.ModMetadata() {
            public String getId() { return id; }
            public String getName() { return id; }
            public Version getVersion() { return version; }
        };
        return Optional.of(() -> meta);
    }
    default Collection<ModContainer> getAllMods() { return List.of(); }
    default EnvType getEnvironmentType() { return EnvType.SERVER; }
    default boolean isDevelopmentEnvironment() { return false; }
    default Path getGameDir() { return Path.of("."); }
    default Path getConfigDir() { return Path.of("config"); }
}
