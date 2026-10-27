package net.minecraft.server.level;

/** Shim stub (generated): names and signatures only, static-ness guessed. */
public class ServerLevel {
    public static net.minecraft.resources.ResourceKey OVERWORLD;
    public boolean addFreshEntity(net.minecraft.world.entity.Entity a0) { return false; }
    public void addFreshEntityWithPassengers(net.minecraft.world.entity.Entity a0) { }
    public boolean addWithUUID(net.minecraft.world.entity.Entity a0) { return false; }
    public boolean areEntitiesLoaded(long a0) { return false; }
    public net.minecraft.world.clock.ServerClockManager clockManager() { return null; }
    public void close() { }
    public boolean destroyBlock(net.minecraft.core.BlockPos a0, boolean a1) { return false; }
    public void destroyBlockProgress(int a0, net.minecraft.core.BlockPos a1, int a2) { }
    public net.minecraft.resources.ResourceKey dimension() { return null; }
    public net.minecraft.world.level.dimension.DimensionType dimensionType() { return null; }
    public net.minecraft.core.Holder dimensionTypeRegistration() { return null; }
    public java.lang.Iterable getAllEntities() { return null; }
    public int getBestNeighborSignal(net.minecraft.core.BlockPos a0) { return 0; }
    public net.minecraft.core.Holder getBiome(net.minecraft.core.BlockPos a0) { return null; }
    public net.minecraft.world.level.block.entity.BlockEntity getBlockEntity(net.minecraft.core.BlockPos a0) { return null; }
    public net.minecraft.world.level.block.state.BlockState getBlockState(net.minecraft.core.BlockPos a0) { return null; }
    public net.minecraft.world.ticks.LevelTicks getBlockTicks() { return null; }
    public int getBrightness(net.minecraft.world.level.LightLayer a0, net.minecraft.core.BlockPos a1) { return 0; }
    public net.minecraft.world.level.chunk.LevelChunk getChunk(int a0, int a1) { return null; }
    public net.minecraft.world.level.chunk.ChunkAccess getChunk(int a0, int a1, net.minecraft.world.level.chunk.status.ChunkStatus a2) { return null; }
    public net.minecraft.world.level.chunk.ChunkAccess getChunk(int a0, int a1, net.minecraft.world.level.chunk.status.ChunkStatus a2, boolean a3) { return null; }
    public net.minecraft.world.level.chunk.ChunkAccess getChunk(net.minecraft.core.BlockPos a0) { return null; }
    public net.minecraft.world.level.chunk.LevelChunk getChunkAt(net.minecraft.core.BlockPos a0) { return null; }
    public net.minecraft.server.level.ServerChunkCache getChunkSource() { return null; }
    public net.minecraft.world.DifficultyInstance getCurrentDifficultyAt(net.minecraft.core.BlockPos a0) { return null; }
    public int getDirectSignalTo(net.minecraft.core.BlockPos a0) { return 0; }
    public java.util.List getEntities(net.minecraft.world.entity.Entity a0, net.minecraft.world.phys.AABB a1) { return null; }
    public java.util.List getEntities(net.minecraft.world.entity.Entity a0, net.minecraft.world.phys.AABB a1, java.util.function.Predicate a2) { return null; }
    public java.util.List getEntities(net.minecraft.world.level.entity.EntityTypeTest a0, java.util.function.Predicate a1) { return null; }
    public java.util.List getEntities(net.minecraft.world.level.entity.EntityTypeTest a0, net.minecraft.world.phys.AABB a1, java.util.function.Predicate a2) { return null; }
    public java.util.List getEntitiesOfClass(java.lang.Class a0, net.minecraft.world.phys.AABB a1, java.util.function.Predicate a2) { return null; }
    public net.minecraft.world.entity.Entity getEntity(int a0) { return null; }
    public net.minecraft.world.entity.Entity getEntity(java.util.UUID a0) { return null; }
    public net.minecraft.world.level.material.FluidState getFluidState(net.minecraft.core.BlockPos a0) { return null; }
    public net.minecraft.world.ticks.LevelTicks getFluidTicks() { return null; }
    public net.minecraft.world.level.gamerules.GameRules getGameRules() { return null; }
    public long getGameTime() { return 0; }
    public net.minecraft.world.level.storage.LevelData getLevelData() { return null; }
    public net.minecraft.world.level.lighting.LevelLightEngine getLightEngine() { return null; }
    public float getLightLevelDependentMagicValue(net.minecraft.core.BlockPos a0) { return 0; }
    public int getMaxLocalRawBrightness(net.minecraft.core.BlockPos a0) { return 0; }
    public int getMaxY() { return 0; }
    public int getMinY() { return 0; }
    public net.minecraft.world.entity.player.Player getNearestPlayer(double a0, double a1, double a2, double a3, java.util.function.Predicate a4) { return null; }
    public long getOverworldClockTime() { return 0; }
    public net.minecraft.world.entity.player.Player getPlayerByUUID(java.util.UUID a0) { return null; }
    public java.util.List getPlayers(java.util.function.Predicate a0) { return null; }
    public net.minecraft.world.entity.ai.village.poi.PoiManager getPoiManager() { return null; }
    public net.minecraft.util.RandomSource getRandom() { return null; }
    public int getSeaLevel() { return 0; }
    public int getSectionIndex(int a0) { return 0; }
    public long getSeed() { return 0; }
    public net.minecraft.server.MinecraftServer getServer() { return null; }
    public net.minecraft.world.level.levelgen.structure.templatesystem.StructureTemplateManager getStructureManager() { return null; }
    public net.minecraft.world.level.border.WorldBorder getWorldBorder() { return null; }
    public boolean hasChunk(int a0, int a1) { return false; }
    public boolean hasChunkAt(net.minecraft.core.BlockPos a0) { return false; }
    public net.minecraft.core.HolderLookup holderLookup(net.minecraft.resources.ResourceKey a0) { return null; }
    public boolean isClientSide() { return false; }
    public boolean isDebug() { return false; }
    public boolean isEmptyBlock(net.minecraft.core.BlockPos a0) { return false; }
    public boolean isInWorldBounds(net.minecraft.core.BlockPos a0) { return false; }
    public boolean isPositionEntityTicking(net.minecraft.core.BlockPos a0) { return false; }
    public boolean isRaining() { return false; }
    public boolean isThundering() { return false; }
    public void levelEvent(net.minecraft.world.entity.Entity a0, int a1, net.minecraft.core.BlockPos a2, int a3) { }
    public boolean mayInteract(net.minecraft.world.entity.Entity a0, net.minecraft.core.BlockPos a1) { return false; }
    public void neighborChanged(net.minecraft.core.BlockPos a0, net.minecraft.world.level.block.Block a1, net.minecraft.world.level.redstone.Orientation a2) { }
    public boolean noCollision(net.minecraft.world.entity.Entity a0) { return false; }
    public boolean noCollision(net.minecraft.world.phys.AABB a0) { return false; }
    public boolean noSave;
    public java.util.List players() { return null; }
    public net.minecraft.world.item.crafting.RecipeManager recipeAccess() { return null; }
    public net.minecraft.core.RegistryAccess registryAccess() { return null; }
    public boolean removeBlock(net.minecraft.core.BlockPos a0, boolean a1) { return false; }
    public void sendBlockUpdated(net.minecraft.core.BlockPos a0, net.minecraft.world.level.block.state.BlockState a1, net.minecraft.world.level.block.state.BlockState a2, int a3) { }
    public int sendParticles(net.minecraft.core.particles.ParticleOptions a0, double a1, double a2, double a3, int a4, double a5, double a6, double a7, double a8) { return 0; }
    public boolean sendParticles(net.minecraft.server.level.ServerPlayer a0, net.minecraft.core.particles.ParticleOptions a1, boolean a2, boolean a3, double a4, double a5, double a6, int a7, double a8, double a9, double a10, double a11) { return false; }
    public boolean setBlock(net.minecraft.core.BlockPos a0, net.minecraft.world.level.block.state.BlockState a1, int a2) { return false; }
    public boolean setBlockAndUpdate(net.minecraft.core.BlockPos a0, net.minecraft.world.level.block.state.BlockState a1) { return false; }
    public void setCurrentlyGenerating(java.util.function.Supplier a0) { }
    public net.minecraft.world.level.StructureManager structureManager() { return null; }
    public boolean tryAddFreshEntityWithPassengers(net.minecraft.world.entity.Entity a0) { return false; }
    public void updateNeighborsAt(net.minecraft.core.BlockPos a0, net.minecraft.world.level.block.Block a1) { }
    public void updateNeighborsAt(net.minecraft.core.BlockPos a0, net.minecraft.world.level.block.Block a1, net.minecraft.world.level.redstone.Orientation a2) { }
    public void updateNeighbourForOutputSignal(net.minecraft.core.BlockPos a0, net.minecraft.world.level.block.Block a1) { }
    public void updateNeighboursOnBlockSet(net.minecraft.core.BlockPos a0, net.minecraft.world.level.block.state.BlockState a1) { }
    public void updatePOIOnBlockStateChange(net.minecraft.core.BlockPos a0, net.minecraft.world.level.block.state.BlockState a1, net.minecraft.world.level.block.state.BlockState a2) { }
}
