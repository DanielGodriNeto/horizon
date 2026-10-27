package org.slf4j;

public final class LoggerFactory {
    private static final Logger NOP = new Logger() {};
    private LoggerFactory() {}
    public static Logger getLogger(String name) { return NOP; }
    public static Logger getLogger(Class<?> c) { return NOP; }
}
