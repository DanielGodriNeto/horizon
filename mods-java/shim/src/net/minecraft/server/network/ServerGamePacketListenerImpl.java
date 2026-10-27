package net.minecraft.server.network;

/** Shim stub (generated): names and signatures only, static-ness guessed. */
public class ServerGamePacketListenerImpl {
    public ServerGamePacketListenerImpl(net.minecraft.server.MinecraftServer a0, net.minecraft.network.Connection a1, net.minecraft.server.level.ServerPlayer a2, net.minecraft.server.network.CommonListenerCookie a3) {}
    public void disconnect(net.minecraft.network.chat.Component a0) { }
    public net.minecraft.server.level.ServerPlayer getPlayer() { return null; }
    public void handleClientCommand(net.minecraft.network.protocol.game.ServerboundClientCommandPacket a0) { }
    public boolean isAcceptingMessages() { return false; }
    public int latency() { return 0; }
    public void onDisconnect(net.minecraft.network.DisconnectionDetails a0) { }
    public net.minecraft.server.level.ServerPlayer player;
    public void resetPosition() { }
    public void send(net.minecraft.network.protocol.Packet a0) { }
    public boolean shouldHandleMessage(net.minecraft.network.protocol.Packet a0) { return false; }
    public void teleport(double a0, double a1, double a2, float a3, float a4) { }
    public void teleport(net.minecraft.world.entity.PositionMoveRotation a0, java.util.Set a1) { }
}
