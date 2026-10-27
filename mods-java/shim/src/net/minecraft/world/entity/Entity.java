package net.minecraft.world.entity;

/** Shim stub (generated): names and signatures only, static-ness guessed. */
public class Entity {
    public Entity(net.minecraft.world.entity.EntityType a0, net.minecraft.world.level.Level a1) {}
    public void absSnapTo(double a0, double a1, double a2, float a3, float a4) { }
    public boolean addTag(java.lang.String a0) { return false; }
    public net.minecraft.core.BlockPos blockPosition() { return null; }
    public boolean canBeCollidedWith(net.minecraft.world.entity.Entity a0) { return false; }
    public boolean canCollideWith(net.minecraft.world.entity.Entity a0) { return false; }
    public boolean canFreeze() { return false; }
    public boolean canSpawnSprintParticle() { return false; }
    public net.minecraft.world.level.ChunkPos chunkPosition() { return null; }
    public net.minecraft.commands.CommandSourceStack createCommandSourceStackForNameResolution(net.minecraft.server.level.ServerLevel a0) { return null; }
    public net.minecraft.world.damagesource.DamageSources damageSources() { return null; }
    public void discard() { }
    public void ejectPassengers() { }
    public java.util.Set entityTags() { return null; }
    public boolean equals(java.lang.Object a0) { return false; }
    public boolean fireImmune() { return false; }
    public int getAirSupply() { return 0; }
    public float getBbWidth() { return 0; }
    public float getBlockExplosionResistance(net.minecraft.world.level.Explosion a0, net.minecraft.world.level.BlockGetter a1, net.minecraft.core.BlockPos a2, net.minecraft.world.level.block.state.BlockState a3, net.minecraft.world.level.material.FluidState a4, float a5) { return 0; }
    public net.minecraft.world.phys.AABB getBoundingBox() { return null; }
    public net.minecraft.network.chat.Component getCustomName() { return null; }
    public net.minecraft.world.phys.Vec3 getDeltaMovement() { return null; }
    public net.minecraft.world.entity.EntityDimensions getDimensions(net.minecraft.world.entity.Pose a0) { return null; }
    public net.minecraft.core.Direction getDirection() { return null; }
    public net.minecraft.network.chat.Component getDisplayName() { return null; }
    public float getEyeHeight() { return 0; }
    public net.minecraft.world.phys.Vec3 getEyePosition() { return null; }
    public net.minecraft.world.phys.Vec3 getEyePosition(float a0) { return null; }
    public net.minecraft.world.entity.Entity getFirstPassenger() { return null; }
    public int getId() { return 0; }
    public net.minecraft.world.phys.Vec3 getLookAngle() { return null; }
    public net.minecraft.network.chat.Component getName() { return null; }
    public net.minecraft.world.phys.Vec3 getPassengerRidingPosition(net.minecraft.world.entity.Entity a0) { return null; }
    public java.util.List getPassengers() { return null; }
    public float getPickRadius() { return 0; }
    public net.minecraft.world.level.material.PushReaction getPistonPushReaction() { return null; }
    public net.minecraft.world.entity.Pose getPose() { return null; }
    public net.minecraft.world.phys.Vec3 getPosition(float a0) { return null; }
    public int getRemainingFireTicks() { return 0; }
    public net.minecraft.world.entity.Entity getRootVehicle() { return null; }
    public java.lang.String getScoreboardName() { return null; }
    public java.lang.String getStringUUID() { return null; }
    public net.minecraft.world.scores.PlayerTeam getTeam() { return null; }
    public int getTicksFrozen() { return 0; }
    public net.minecraft.world.entity.EntityType getType() { return null; }
    public java.util.UUID getUUID() { return null; }
    public net.minecraft.world.entity.Entity getVehicle() { return null; }
    public net.minecraft.world.phys.Vec3 getViewVector(float a0) { return null; }
    public float getViewXRot(float a0) { return 0; }
    public float getViewYRot(float a0) { return 0; }
    public double getX() { return 0; }
    public float getXRot() { return 0; }
    public double getY() { return 0; }
    public float getYHeadRot() { return 0; }
    public float getYRot() { return 0; }
    public double getZ() { return 0; }
    public boolean hasCustomName() { return false; }
    public int hashCode() { return 0; }
    public boolean hurtMarked;
    public net.minecraft.world.InteractionResult interact(net.minecraft.world.entity.player.Player a0, net.minecraft.world.InteractionHand a1, net.minecraft.world.phys.Vec3 a2) { return null; }
    public boolean isAlive() { return false; }
    public boolean isAttackable() { return false; }
    public boolean isDescending() { return false; }
    public boolean isFullyFrozen() { return false; }
    public boolean isInLiquid() { return false; }
    public boolean isInvulnerable() { return false; }
    public boolean isNoGravity() { return false; }
    public boolean isOnFire() { return false; }
    public boolean isPassenger() { return false; }
    public boolean isPickable() { return false; }
    public boolean isRemoved() { return false; }
    public boolean isShiftKeyDown() { return false; }
    public boolean isSilent() { return false; }
    public boolean isSpectator() { return false; }
    public boolean isSprinting() { return false; }
    public boolean isSwimming() { return false; }
    public boolean isVehicle() { return false; }
    public void kill(net.minecraft.server.level.ServerLevel a0) { }
    public net.minecraft.world.level.Level level() { return null; }
    public void load(net.minecraft.world.level.storage.ValueInput a0) { }
    public float mirror(net.minecraft.world.level.block.Mirror a0) { return 0; }
    public boolean needsSync;
    public boolean noPhysics;
    public void onExplosionHit(net.minecraft.world.entity.Entity a0) { }
    public boolean onGround() { return false; }
    public net.minecraft.world.phys.Vec3 position() { return null; }
    public void push(double a0, double a1, double a2) { }
    public void push(net.minecraft.world.phys.Vec3 a0) { }
    public net.minecraft.core.RegistryAccess registryAccess() { return null; }
    public boolean removeTag(java.lang.String a0) { return false; }
    public void rideTick() { }
    public float rotate(net.minecraft.world.level.block.Rotation a0) { return 0; }
    public boolean save(net.minecraft.world.level.storage.ValueOutput a0) { return false; }
    public void saveWithoutId(net.minecraft.world.level.storage.ValueOutput a0) { }
    public void setAirSupply(int a0) { }
    public void setCustomName(net.minecraft.network.chat.Component a0) { }
    public void setCustomNameVisible(boolean a0) { }
    public void setDeltaMovement(double a0, double a1, double a2) { }
    public void setDeltaMovement(net.minecraft.world.phys.Vec3 a0) { }
    public void setId(int a0) { }
    public void setInvulnerable(boolean a0) { }
    public void setNoGravity(boolean a0) { }
    public void setPos(double a0, double a1, double a2) { }
    public void setRemainingFireTicks(int a0) { }
    public void setSilent(boolean a0) { }
    public void setTicksFrozen(int a0) { }
    public void setUUID(java.util.UUID a0) { }
    public void setXRot(float a0) { }
    public void setYHeadRot(float a0) { }
    public void setYRot(float a0) { }
    public boolean shouldBlockExplode(net.minecraft.world.level.Explosion a0, net.minecraft.world.level.BlockGetter a1, net.minecraft.core.BlockPos a2, net.minecraft.world.level.block.state.BlockState a3, float a4) { return false; }
    public void snapTo(double a0, double a1, double a2, float a3, float a4) { }
    public void snapTo(net.minecraft.core.BlockPos a0, float a1, float a2) { }
    public boolean startRiding(net.minecraft.world.entity.Entity a0, boolean a1, boolean a2) { return false; }
    public void stopRiding() { }
    public void thunderHit(net.minecraft.server.level.ServerLevel a0, net.minecraft.world.entity.LightningBolt a1) { }
    public void tick() { }
    public int tickCount;
    public float xRotO;
    public double xo;
    public float yRotO;
    public double yo;
    public double zo;
}
