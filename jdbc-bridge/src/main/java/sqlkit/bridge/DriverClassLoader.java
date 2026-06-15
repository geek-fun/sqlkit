package sqlkit.bridge;

import java.io.File;
import java.net.URL;
import java.net.URLClassLoader;
import java.util.List;

/**
 * Isolated URLClassLoader for JDBC drivers.
 * <p>
 * Each driver type gets its own classloader with a null parent,
 * preventing class conflicts between different driver versions
 * that might otherwise occur via {@link java.sql.DriverManager}.
 * <p>
 * Driver JARs are searched first, then falls back to the system
 * classloader for JDK classes only.
 */
public class DriverClassLoader extends URLClassLoader {

    /**
     * Create an isolated classloader loading from the given JAR paths.
     *
     * @param jarPaths list of absolute paths to JDBC driver JAR files
     */
    public DriverClassLoader(List<String> jarPaths) {
        super(
            jarPaths.stream()
                .map(p -> {
                    try {
                        return new File(p).toURI().toURL();
                    } catch (Exception e) {
                        throw new RuntimeException("Invalid JAR path: " + p, e);
                    }
                })
                .toArray(URL[]::new),
            null // null parent = isolated from other driver classes
        );
    }

    /**
     * Load a class: try driver JARs first, fall back to system
     * classloader for JDK / standard library classes.
     */
    @Override
    public Class<?> loadClass(String name) throws ClassNotFoundException {
        synchronized (getClassLoadingLock(name)) {
            // Already loaded by this loader?
            Class<?> c = findLoadedClass(name);
            if (c != null) {
                return c;
            }

            try {
                // Try our own URLs first (driver classes)
                return findClass(name);
            } catch (ClassNotFoundException e) {
                // Fall back to system classloader for JDK classes
                return Class.forName(name, true, ClassLoader.getSystemClassLoader());
            }
        }
    }
}
