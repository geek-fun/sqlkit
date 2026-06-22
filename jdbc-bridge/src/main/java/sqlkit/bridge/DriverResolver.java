package sqlkit.bridge;

import okhttp3.OkHttpClient;
import okhttp3.Request;
import okhttp3.Response;

import java.io.*;
import java.nio.file.*;
import java.util.concurrent.TimeUnit;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

/**
 * Resolves JDBC driver JARs from Maven Central.
 * 
 * Uses the maven-metadata.xml approach to resolve LATEST,
 * then downloads the JAR to a local cache directory.
 * Requires only okhttp3 — no heavyweight Maven Resolver/Aether deps.
 */
public class DriverResolver {
    
    private static final String MAVEN_CENTRAL = "https://repo1.maven.org/maven2";
    private static final String DRIVERS_CACHE = getDriversCacheDir();
    
    private static final OkHttpClient HTTP_CLIENT = new OkHttpClient.Builder()
        .connectTimeout(15, TimeUnit.SECONDS)
        .readTimeout(30, TimeUnit.SECONDS)
        .followRedirects(true)
        .build();
    
    private static String getDriversCacheDir() {
        String home = System.getProperty("user.home");
        return home + "/.sqlkit/jdbc-bridge/drivers";
    }
    
    /**
     * Resolve the latest version of a Maven artifact and download it.
     *
     * @param mavenGroup     e.g. "com.h2database"
     * @param mavenArtifact  e.g. "h2"
     * @param versionCap     Optional max version to cap against. Null means resolve LATEST.
     * @param classifier     Optional Maven classifier (e.g. "standalone"). Null means no classifier.
     * @param progressCb     Callback invoked with (downloaded, total) during JAR download, or null.
     * @return DriverResult with path to the cached JAR and resolved version
     */
    public static DriverResult resolve(String mavenGroup, String mavenArtifact, 
                                        String versionCap, String classifier,
                                        ProgressCallback progressCb) throws Exception {
        // 1. Fetch maven-metadata.xml
        String metadataUrl = String.format("%s/%s/%s/maven-metadata.xml",
            MAVEN_CENTRAL, mavenGroup.replace('.', '/'), mavenArtifact);
        
        Request request = new Request.Builder()
            .url(metadataUrl)
            .addHeader("User-Agent", "SQLKit/1.0")
            .build();
        
        Response response = HTTP_CLIENT.newCall(request).execute();
        if (!response.isSuccessful()) {
            throw new Exception("Failed to fetch Maven metadata: HTTP " + response.code() + " for " + metadataUrl);
        }
        
        String metadata = response.body() != null ? response.body().string() : "";
        response.close();
        
        // 2. Parse latest version from metadata XML
        String latestVersion = parseLatestVersion(metadata);
        if (latestVersion == null) {
            // Fall back to <release> element
            latestVersion = parseReleaseVersion(metadata);
        }
        if (latestVersion == null) {
            throw new Exception("Could not determine latest version from Maven metadata");
        }
        
        // 3. Apply version cap if specified
        if (versionCap != null && !versionCap.isEmpty()) {
            // Simple comparison: if latest > cap, use cap
            if (compareVersions(latestVersion, versionCap) > 0) {
                latestVersion = versionCap;
            }
        }
        
        // 4. Build download URL and destination
        String classifierSuffix = (classifier != null && !classifier.isEmpty()) 
            ? "-" + classifier : "";
        String jarFilename = mavenArtifact + "-" + latestVersion + classifierSuffix + ".jar";
        Path destPath = Paths.get(DRIVERS_CACHE, mavenArtifact, jarFilename);
        
        // 5. Download if not cached
        if (!Files.exists(destPath)) {
            Files.createDirectories(destPath.getParent());
            
            String downloadUrl = String.format("%s/%s/%s/%s/%s",
                MAVEN_CENTRAL, mavenGroup.replace('.', '/'), mavenArtifact,
                latestVersion, jarFilename);
            
            request = new Request.Builder()
                .url(downloadUrl)
                .addHeader("User-Agent", "SQLKit/1.0")
                .build();
            
            response = HTTP_CLIENT.newCall(request).execute();
            if (!response.isSuccessful()) {
                throw new Exception("Failed to download JAR: HTTP " + response.code() + " for " + downloadUrl);
            }
            if (response.body() == null) {
                throw new Exception("Empty response body when downloading JAR from " + downloadUrl);
            }
            
            long contentLength = response.body().contentLength();
            long totalBytes = contentLength > 0 ? contentLength : 5_000_000L;
            long downloadedBytes = 0L;
            
            try (InputStream in = new BufferedInputStream(response.body().byteStream());
                 OutputStream out = Files.newOutputStream(destPath)) {
                byte[] buffer = new byte[4096];
                int bytesRead;
                while ((bytesRead = in.read(buffer)) != -1) {
                    out.write(buffer, 0, bytesRead);
                    downloadedBytes += bytesRead;
                    if (progressCb != null) {
                        progressCb.onProgress(downloadedBytes, totalBytes);
                    }
                }
            }
        }
        
        return new DriverResult(destPath.toAbsolutePath().toString(), latestVersion);
    }
    
    /**
     * Parse the latest version from <versioning><latest> element in maven-metadata.xml.
     */
    private static String parseLatestVersion(String xml) {
        Pattern p = Pattern.compile("<latest>(.+?)</latest>");
        Matcher m = p.matcher(xml);
        return m.find() ? m.group(1).trim() : null;
    }
    
    /**
     * Parse the release version from <versioning><release> element.
     */
    private static String parseReleaseVersion(String xml) {
        Pattern p = Pattern.compile("<release>(.+?)</release>");
        Matcher m = p.matcher(xml);
        return m.find() ? m.group(1).trim() : null;
    }
    
    /**
     * Simple version comparison (handles dotted versions like "2.2.224" or "21.15.0.0").
     * Returns negative if a < b, positive if a > b, 0 if equal.
     */
    private static int compareVersions(String a, String b) {
        String[] partsA = a.split("\\.");
        String[] partsB = b.split("\\.");
        int len = Math.max(partsA.length, partsB.length);
        for (int i = 0; i < len; i++) {
            int numA = i < partsA.length ? tryParseInt(partsA[i]) : 0;
            int numB = i < partsB.length ? tryParseInt(partsB[i]) : 0;
            if (numA != numB) return Integer.compare(numA, numB);
        }
        return 0;
    }
    
    private static int tryParseInt(String s) {
        try {
            // Handle version parts like "0" from "1.0.0"
            // Also handle parts like "1-SNAPSHOT" by taking the numeric prefix
            s = s.replaceAll("-.*$", "");
            if (s.isEmpty()) return 0;
            // Handle parts like "21.0.11+10" by stripping +suffix
            int plusIdx = s.indexOf('+');
            if (plusIdx > 0) s = s.substring(0, plusIdx);
            return Integer.parseInt(s);
        } catch (NumberFormatException e) {
            return 0;
        }
    }
    
    /**
     * Callback interface for tracking JAR download progress.
     */
    public interface ProgressCallback {
        void onProgress(long downloaded, long total);
    }

    /**
     * Result of a driver resolution.
     */
    public static class DriverResult {
        private final String jarPath;
        private final String resolvedVersion;
        
        public DriverResult(String jarPath, String resolvedVersion) {
            this.jarPath = jarPath;
            this.resolvedVersion = resolvedVersion;
        }
        
        public String getJarPath() { return jarPath; }
        public String getResolvedVersion() { return resolvedVersion; }
    }
}
