package org.apache.logging.log4j;

public final class Level {
    public static final Level OFF = new Level("OFF"), FATAL = new Level("FATAL"), ERROR = new Level("ERROR"),
            WARN = new Level("WARN"), INFO = new Level("INFO"), DEBUG = new Level("DEBUG"), TRACE = new Level("TRACE"),
            ALL = new Level("ALL");
    private final String name;
    private Level(String name) { this.name = name; }
    public String name() { return name; }
    @Override public String toString() { return name; }
}
