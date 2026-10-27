package net.minecraft.world.level.block.state;

/** Shim stub: carries the block id (the real class reaches it through the Block). */
public class BlockState {
    private final String blockId;

    public BlockState(String blockId) { this.blockId = blockId; }

    public String blockId() { return blockId; }
}
