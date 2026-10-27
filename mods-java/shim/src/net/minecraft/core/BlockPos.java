package net.minecraft.core;

/** Shim stub: only the coordinates. */
public class BlockPos {
    private final int x, y, z;

    public BlockPos(int x, int y, int z) { this.x = x; this.y = y; this.z = z; }

    public int getX() { return x; }
    public int getY() { return y; }
    public int getZ() { return z; }
}
