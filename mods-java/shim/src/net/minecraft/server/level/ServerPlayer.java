package net.minecraft.server.level;

/** Shim stub (generated): names and signatures only, static-ness guessed. */
public class ServerPlayer {
    public void attack(net.minecraft.world.entity.Entity a0) { }
    public boolean blockActionRestricted(net.minecraft.world.level.Level a0, net.minecraft.core.BlockPos a1, net.minecraft.world.level.GameType a2) { return false; }
    public net.minecraft.core.BlockPos blockPosition() { return null; }
    public boolean canChatInColor() { return false; }
    public net.minecraft.server.level.ClientInformation clientInformation() { return null; }
    public void closeContainer() { }
    public net.minecraft.commands.CommandSource commandSource() { return null; }
    public net.minecraft.server.network.ServerGamePacketListenerImpl connection;
    public net.minecraft.world.inventory.AbstractContainerMenu containerMenu;
    public net.minecraft.commands.CommandSourceStack createCommandSourceStack() { return null; }
    public void die(net.minecraft.world.damagesource.DamageSource a0) { }
    public double distanceToSqr(net.minecraft.world.entity.Entity a0) { return 0; }
    public double distanceToSqr(net.minecraft.world.phys.Vec3 a0) { return 0; }
    public net.minecraft.world.entity.item.ItemEntity drop(net.minecraft.world.item.ItemStack a0, boolean a1, boolean a2) { return null; }
    public void drop(boolean a0) { }
    public boolean equals(java.lang.Object a0) { return false; }
    public int experienceLevel;
    public float experienceProgress;
    public net.minecraft.server.level.ServerPlayerGameMode gameMode;
    public net.minecraft.world.entity.player.Abilities getAbilities() { return null; }
    public net.minecraft.world.phys.AABB getBoundingBox() { return null; }
    public net.minecraft.world.entity.Entity getCamera() { return null; }
    public net.minecraft.network.chat.RemoteChatSession getChatSession() { return null; }
    public net.minecraft.network.chat.Component getDisplayName() { return null; }
    public net.minecraft.world.inventory.PlayerEnderChestContainer getEnderChestInventory() { return null; }
    public net.minecraft.network.syncher.SynchedEntityData getEntityData() { return null; }
    public net.minecraft.world.phys.Vec3 getEyePosition() { return null; }
    public float getHealth() { return 0; }
    public int getId() { return 0; }
    public net.minecraft.world.entity.player.Inventory getInventory() { return null; }
    public net.minecraft.world.item.ItemStack getItemInHand(net.minecraft.world.InteractionHand a0) { return null; }
    public net.minecraft.core.SectionPos getLastSectionPos() { return null; }
    public net.minecraft.network.chat.Component getName() { return null; }
    public net.minecraft.world.phys.Vec2 getRotationVector() { return null; }
    public java.lang.String getScoreboardName() { return null; }
    public net.minecraft.stats.ServerStatsCounter getStats() { return null; }
    public java.lang.String getStringUUID() { return null; }
    public java.util.UUID getUUID() { return null; }
    public net.minecraft.world.item.ItemStack getUseItem() { return null; }
    public net.minecraft.world.InteractionHand getUsedItemHand() { return null; }
    public net.minecraft.world.entity.Entity getVehicle() { return null; }
    public double getX() { return 0; }
    public float getXRot() { return 0; }
    public double getY() { return 0; }
    public float getYHeadRot() { return 0; }
    public float getYRot() { return 0; }
    public double getZ() { return 0; }
    public void hasChangedDimension() { }
    public net.minecraft.world.InteractionResult interactOn(net.minecraft.world.entity.Entity a0, net.minecraft.world.InteractionHand a1, net.minecraft.world.phys.Vec3 a2) { return null; }
    public net.minecraft.world.inventory.InventoryMenu inventoryMenu;
    public boolean isChangingDimension() { return false; }
    public boolean isCreative() { return false; }
    public boolean isShiftKeyDown() { return false; }
    public boolean isSleeping() { return false; }
    public boolean isSpectator() { return false; }
    public boolean isUsingItem() { return false; }
    public void jumpFromGround() { }
    public void kill(net.minecraft.server.level.ServerLevel a0) { }
    public net.minecraft.server.level.ServerLevel level() { return null; }
    public net.minecraft.server.players.NameAndId nameAndId() { return null; }
    public boolean onClimbable() { return false; }
    public void onEquipItem(net.minecraft.world.entity.EquipmentSlot a0, net.minecraft.world.item.ItemStack a1, net.minecraft.world.item.ItemStack a2) { }
    public boolean onGround() { return false; }
    public void onUpdateAbilities() { }
    public java.util.OptionalInt openMenu(net.minecraft.world.MenuProvider a0) { return null; }
    public net.minecraft.server.permissions.PermissionSet permissions() { return null; }
    public net.minecraft.world.phys.Vec3 position() { return null; }
    public net.minecraft.core.RegistryAccess registryAccess() { return null; }
    public void releaseUsingItem() { }
    public void resetAttackStrengthTicker() { }
    public void resetLastActionTime() { }
    public void sendOverlayMessage(net.minecraft.network.chat.Component a0) { }
    public void sendSystemMessage(net.minecraft.network.chat.Component a0) { }
    public void sendSystemMessage(net.minecraft.network.chat.Component a0, boolean a1) { }
    public boolean setGameMode(net.minecraft.world.level.GameType a0) { return false; }
    public void setItemInHand(net.minecraft.world.InteractionHand a0, net.minecraft.world.item.ItemStack a1) { }
    public void setJumping(boolean a0) { }
    public void setShiftKeyDown(boolean a0) { }
    public void setSprinting(boolean a0) { }
    public void setXRot(float a0) { }
    public void setYRot(float a0) { }
    public boolean startRiding(net.minecraft.world.entity.Entity a0, boolean a1, boolean a2) { return false; }
    public void stopRiding() { }
    public void swing(net.minecraft.world.InteractionHand a0) { }
    public boolean swinging;
    public net.minecraft.server.level.ServerPlayer teleport(net.minecraft.world.level.portal.TeleportTransition a0) { return null; }
    public boolean teleportTo(net.minecraft.server.level.ServerLevel a0, double a1, double a2, double a3, java.util.Set a4, float a5, float a6, boolean a7) { return false; }
    public void tick() { }
    public int totalExperience;
    public boolean tryToStartFallFlying() { return false; }
    public float xxa;
    public float yHeadRot;
    public float zza;
}
