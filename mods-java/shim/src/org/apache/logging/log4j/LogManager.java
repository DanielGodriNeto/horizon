package org.apache.logging.log4j;

public final class LogManager {
    private static final Logger NOP = new Logger() {};
    private LogManager() {}
    public static Logger getLogger() { return NOP; }
    public static Logger getLogger(String name) { return NOP; }
    public static Logger getLogger(Class<?> c) { return NOP; }
    public static Logger getLogger(Object o) { return NOP; }
    public static Logger getRootLogger() { return NOP; }
}
