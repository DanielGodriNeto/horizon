package horizon;

import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.nio.charset.StandardCharsets;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;
import java.util.function.Function;
import net.fabricmc.fabric.api.event.player.PlayerBlockBreakEvents;
import net.minecraft.core.BlockPos;
import net.minecraft.world.entity.player.Player;
import net.minecraft.world.item.Item;
import net.minecraft.world.level.Level;
import net.minecraft.world.level.block.state.BlockState;

/**
 * The mod side of the ring (mods-java/PROTOCOLO.md, "Mensagens do host"). The Sidecar calls it by
 * reflection: {@link #open} runs the mod and returns its registrations, {@link #handler} answers events.
 */
public final class LiveHost {
    static final byte REGISTER_ITEM = 1;
    static final byte BLOCK_BREAK = 2;
    static final byte REPLY = 3;

    private static final Level WORLD = new Level();

    private LiveHost() {}

    /** Runs the mod's main entrypoints; one REGISTER_ITEM record per item: type, u16 id length, id, item json. */
    public static List<byte[]> open(String modJar) throws Exception {
        ModHost.load(Path.of(modJar));
        List<byte[]> out = new ArrayList<>();
        for (ShimHost.Entry e : ShimHost.entries()) {
            if (e.registry().equals("item") && e.value() instanceof Item item) {
                byte[] id = (e.id().getNamespace() + ":" + e.id().getPath()).getBytes(StandardCharsets.UTF_8);
                byte[] json = ShimHost.itemJson(item).getBytes(StandardCharsets.UTF_8);
                ByteBuffer b = le(1 + 2 + id.length + json.length);
                b.put(REGISTER_ITEM).putShort((short) id.length).put(id).put(json);
                out.add(b.array());
            }
        }
        return out;
    }

    /** Event in, reply out (or null). BLOCK_BREAK: type, u64 seq, i32 x, y, z, block id. REPLY: type, u64 seq, messages. */
    public static Function<byte[], byte[]> handler() {
        return LiveHost::onEvent;
    }

    private static byte[] onEvent(byte[] msg) {
        ByteBuffer in = ByteBuffer.wrap(msg).order(ByteOrder.LITTLE_ENDIAN);
        if (in.get() != BLOCK_BREAK) return null;
        long seq = in.getLong();
        BlockPos pos = new BlockPos(in.getInt(), in.getInt(), in.getInt());
        byte[] id = new byte[in.remaining()];
        in.get(id);
        BlockState state = new BlockState(new String(id, StandardCharsets.UTF_8));
        Player player = new Player();
        for (PlayerBlockBreakEvents.After cb : PlayerBlockBreakEvents.AFTER.listeners()) {
            cb.afterBlockBreak(WORLD, player, pos, state, null);
        }
        byte[] text = player.takeMessages().getBytes(StandardCharsets.UTF_8);
        return le(1 + 8 + text.length).put(REPLY).putLong(seq).put(text).array();
    }

    private static ByteBuffer le(int n) {
        return ByteBuffer.allocate(n).order(ByteOrder.LITTLE_ENDIAN);
    }
}
