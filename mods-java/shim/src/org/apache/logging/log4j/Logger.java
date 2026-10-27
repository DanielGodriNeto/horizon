package org.apache.logging.log4j;

public interface Logger {
    default void trace(String m) {}
    default void trace(Object m) {}
    default void trace(String m, Object p0) {}
    default void trace(String m, Object p0, Object p1) {}
    default void trace(String m, Object... p) {}
    default void trace(String m, Throwable t) {}
    default void trace(Throwable t) {}
    default void debug(String m) {}
    default void debug(Object m) {}
    default void debug(String m, Object p0) {}
    default void debug(String m, Object p0, Object p1) {}
    default void debug(String m, Object... p) {}
    default void debug(String m, Throwable t) {}
    default void debug(Throwable t) {}
    default void info(String m) {}
    default void info(Object m) {}
    default void info(String m, Object p0) {}
    default void info(String m, Object p0, Object p1) {}
    default void info(String m, Object... p) {}
    default void info(String m, Throwable t) {}
    default void info(Throwable t) {}
    default void warn(String m) {}
    default void warn(Object m) {}
    default void warn(String m, Object p0) {}
    default void warn(String m, Object p0, Object p1) {}
    default void warn(String m, Object... p) {}
    default void warn(String m, Throwable t) {}
    default void warn(Throwable t) {}
    default void error(String m) {}
    default void error(Object m) {}
    default void error(String m, Object p0) {}
    default void error(String m, Object p0, Object p1) {}
    default void error(String m, Object... p) {}
    default void error(String m, Throwable t) {}
    default void error(Throwable t) {}
    default void fatal(String m) {}
    default void fatal(Object m) {}
    default void fatal(String m, Object p0) {}
    default void fatal(String m, Object p0, Object p1) {}
    default void fatal(String m, Object... p) {}
    default void fatal(String m, Throwable t) {}
    default void fatal(Throwable t) {}
    default boolean isEnabled(Level level) { return false; }
    default void log(Level level, String m) {}
    default void log(Level level, String m, Object... p) {}
    default void log(Level level, String m, Throwable t) {}
    default boolean isDebugEnabled() { return false; }
    default boolean isTraceEnabled() { return false; }
    default boolean isInfoEnabled() { return false; }
    default boolean isWarnEnabled() { return false; }
    default boolean isErrorEnabled() { return false; }
    default String getName() { return "shim"; }
}
