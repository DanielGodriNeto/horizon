package horizon;

import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.nio.charset.StandardCharsets;
import java.nio.file.Path;
import com.mojang.brigadier.Command;
import com.mojang.brigadier.CommandDispatcher;
import com.mojang.brigadier.context.CommandContext;
import java.util.ArrayList;
import java.util.List;
import java.util.function.Function;
import net.fabricmc.fabric.api.command.v2.CommandRegistrationCallback;
import net.fabricmc.fabric.api.event.lifecycle.v1.ServerTickEvents;
import net.fabricmc.fabric.api.event.player.PlayerBlockBreakEvents;
import net.minecraft.commands.CommandBuildContext;
import net.minecraft.commands.CommandSourceStack;
import net.minecraft.commands.Commands;
import net.minecraft.core.BlockPos;
import net.minecraft.server.MinecraftServer;
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
    static final byte TICK = 4;
    static final byte COMMAND = 5;
    static final byte REGISTER_COMMAND = 6;

    private static final Level WORLD = new Level();
    private static final MinecraftServer SERVER = new MinecraftServer();
    private static final CommandDispatcher<CommandSourceStack> COMMANDS = new CommandDispatcher<>();

    private LiveHost() {}

    /**
     * Runs the mod's main entrypoints; one REGISTER_ITEM record per item (type, u16 id length, id, item json) and one
     * REGISTER_COMMAND per command word (type, name).
     */
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
        for (CommandRegistrationCallback cb : CommandRegistrationCallback.EVENT.listeners()) {
            cb.register(COMMANDS, new CommandBuildContext(), Commands.CommandSelection.INTEGRATED);
        }
        for (String name : COMMANDS.commands().keySet()) {
            out.add(le(1 + name.length()).put(REGISTER_COMMAND).put(name.getBytes(StandardCharsets.UTF_8)).array());
        }
        return out;
    }

    /**
     * Event in, reply out (or null). BLOCK_BREAK: type, u64 seq, i32 x, y, z, block id. TICK: type, u64 seq, u64 tick.
     * COMMAND: type, u64 seq, line. REPLY: type, u64 seq, messages.
     */
    public static Function<byte[], byte[]> handler() {
        return LiveHost::onEvent;
    }

    private static byte[] onEvent(byte[] msg) {
        ByteBuffer in = ByteBuffer.wrap(msg).order(ByteOrder.LITTLE_ENDIAN);
        byte type = in.get();
        long seq = in.getLong();
        String text = switch (type) {
            case BLOCK_BREAK -> blockBreak(in);
            case TICK -> tick(in.getLong());
            case COMMAND -> command(rest(in));
            default -> null;
        };
        if (text == null) return null;
        byte[] bytes = text.getBytes(StandardCharsets.UTF_8);
        return le(1 + 8 + bytes.length).put(REPLY).putLong(seq).put(bytes).array();
    }

    private static String blockBreak(ByteBuffer in) {
        BlockPos pos = new BlockPos(in.getInt(), in.getInt(), in.getInt());
        BlockState state = new BlockState(rest(in));
        Player player = new Player();
        for (PlayerBlockBreakEvents.After cb : PlayerBlockBreakEvents.AFTER.listeners()) {
            cb.afterBlockBreak(WORLD, player, pos, state, null);
        }
        return join(player.takeMessages(), Outbox.take());
    }

    private static String tick(long tick) {
        SERVER.setTickCount((int) tick);
        for (ServerTickEvents.EndTick cb : ServerTickEvents.END_SERVER_TICK.listeners()) {
            cb.onEndTick(SERVER);
        }
        return Outbox.take();
    }

    /** The first word picks the command; a word no mod registered answers with nothing. */
    private static String command(String line) {
        String word = line.split(" ", 2)[0];
        Command<CommandSourceStack> run = COMMANDS.commands().get(word);
        if (run == null) return "";
        CommandSourceStack source = new CommandSourceStack();
        run.run(new CommandContext<>(source));
        return join(source.takeMessages(), Outbox.take());
    }

    private static String rest(ByteBuffer in) {
        byte[] bytes = new byte[in.remaining()];
        in.get(bytes);
        return new String(bytes, StandardCharsets.UTF_8);
    }

    private static String join(String a, String b) {
        if (a.isEmpty()) return b;
        return b.isEmpty() ? a : a + "\n" + b;
    }

    private static ByteBuffer le(int n) {
        return ByteBuffer.allocate(n).order(ByteOrder.LITTLE_ENDIAN);
    }
}
