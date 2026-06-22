package sqlkit.bridge;

import java.io.*;
import java.nio.file.Files;
import java.nio.file.Path;
import java.security.KeyStore;
import java.security.PrivateKey;
import java.security.cert.Certificate;
import java.security.cert.CertificateFactory;
import java.security.cert.X509Certificate;
import java.security.spec.PKCS8EncodedKeySpec;
import java.util.Base64;
import java.util.Properties;

public class SslPropertyMapper {
    private static final String KEYSTORE_PASSWORD = "changeit";

    public static void applySslProperties(String driverClass, String jdbcUrl,
            String sslMode, String sslCaCert, String sslClientCert,
            String sslClientKey, boolean trustServerCertificate,
            Properties props) {
        if (sslMode == null || sslMode.equals("disable")) return;

        switch (driverClass) {
            case "org.postgresql.Driver":
                props.setProperty("sslmode", mapToPostgresSslMode(sslMode));
                if (sslCaCert != null) props.setProperty("sslrootcert", sslCaCert);
                if (sslClientCert != null) props.setProperty("sslcert", sslClientCert);
                if (sslClientKey != null) props.setProperty("sslkey", sslClientKey);
                break;
            case "com.mysql.cj.jdbc.Driver":
            case "com.mysql.jdbc.Driver":
                props.setProperty("sslMode", mapToMysqlSslMode(sslMode));
                if (sslCaCert != null) {
                    String p12 = convertPemCaCertToPkcs12(sslCaCert);
                    if (p12 != null) {
                        props.setProperty("trustCertificateKeyStoreUrl", "file:" + p12);
                        props.setProperty("trustCertificateKeyStoreType", "PKCS12");
                        props.setProperty("trustCertificateKeyStorePassword", KEYSTORE_PASSWORD);
                    }
                }
                if (sslClientCert != null && sslClientKey != null) {
                    String p12 = convertPemClientCertToPkcs12(sslClientCert, sslClientKey);
                    if (p12 != null) {
                        props.setProperty("clientCertificateKeyStoreUrl", "file:" + p12);
                        props.setProperty("clientCertificateKeyStoreType", "PKCS12");
                        props.setProperty("clientCertificateKeyStorePassword", KEYSTORE_PASSWORD);
                    }
                }
                break;
            case "com.microsoft.sqlserver.jdbc.SQLServerDriver":
                props.setProperty("encrypt", "true");
                props.setProperty("trustServerCertificate",
                    String.valueOf(trustServerCertificate || sslMode.equals("prefer") || sslMode.equals("require")));
                if (sslCaCert != null && (sslMode.equals("verify-ca") || sslMode.equals("verify-full"))) {
                    String p12 = convertPemCaCertToPkcs12(sslCaCert);
                    if (p12 != null) {
                        props.setProperty("trustStore", p12);
                        props.setProperty("trustStoreType", "PKCS12");
                        props.setProperty("trustStorePassword", KEYSTORE_PASSWORD);
                    }
                }
                break;
            case "oracle.jdbc.OracleDriver":
                props.setProperty("oracle.net.ssl", "true");
                if (sslCaCert != null) {
                    String p12 = convertPemCaCertToPkcs12(sslCaCert);
                    if (p12 != null) {
                        props.setProperty("oracle.net.ssl_truststore", p12);
                        props.setProperty("oracle.net.ssl_truststore_type", "PKCS12");
                        props.setProperty("oracle.net.ssl_truststore_password", KEYSTORE_PASSWORD);
                    }
                }
                if (sslClientCert != null && sslClientKey != null) {
                    String p12 = convertPemClientCertToPkcs12(sslClientCert, sslClientKey);
                    if (p12 != null) {
                        props.setProperty("oracle.net.ssl_keystore", p12);
                        props.setProperty("oracle.net.ssl_keystore_type", "PKCS12");
                        props.setProperty("oracle.net.ssl_keystore_password", KEYSTORE_PASSWORD);
                    }
                }
                props.setProperty("oracle.net.ssl_server_dn_match",
                    sslMode.equals("verify-full") ? "true" : "false");
                break;
            case "net.snowflake.client.jdbc.SnowflakeDriver":
                props.setProperty("ssl", "on");
                break;
            case "com.ibm.db2.jcc.DB2Driver":
                props.setProperty("sslConnection", "true");
                if (sslCaCert != null) {
                    String p12 = convertPemCaCertToPkcs12(sslCaCert);
                    if (p12 != null) {
                        props.setProperty("sslTrustStoreLocation", p12);
                        props.setProperty("sslTrustStorePassword", KEYSTORE_PASSWORD);
                    }
                }
                break;
            case "org.apache.hive.jdbc.HiveDriver":
                props.setProperty("ssl", "true");
                if (sslCaCert != null) {
                    String p12 = convertPemCaCertToPkcs12(sslCaCert);
                    if (p12 != null) {
                        props.setProperty("sslTrustStore", p12);
                        props.setProperty("sslTrustStorePassword", KEYSTORE_PASSWORD);
                    }
                }
                break;
            case "com.teradata.jdbc.TeraDriver":
                props.setProperty("TLS", "on");
                break;
            case "com.sap.db.jdbc.Driver":
                props.setProperty("encrypt", "true");
                break;
            case "com.vertica.jdbc.Driver":
                props.setProperty("ssl", "true");
                if (sslMode.equals("verify-full")) props.setProperty("ssl_hostname_verify", "true");
                break;
            default:
                props.setProperty("ssl", "true");
                break;
        }
    }

    private static String convertPemCaCertToPkcs12(String pemPath) {
        try {
            CertificateFactory cf = CertificateFactory.getInstance("X.509");
            java.util.Collection<? extends Certificate> caCerts;
            try (InputStream is = new FileInputStream(pemPath)) {
                caCerts = cf.generateCertificates(is);
            }
            KeyStore ks = KeyStore.getInstance("PKCS12");
            ks.load(null, null);
            int i = 0;
            for (Certificate cert : caCerts) {
                ks.setCertificateEntry("ca-cert-" + i, cert);
                i++;
            }
            Path tempFile = Files.createTempFile("sqlkit-ca-", ".p12");
            try (OutputStream os = Files.newOutputStream(tempFile)) {
                ks.store(os, KEYSTORE_PASSWORD.toCharArray());
            }
            tempFile.toFile().deleteOnExit();
            return tempFile.toAbsolutePath().toString();
        } catch (Exception e) {
            System.err.println("Failed to convert CA cert to PKCS12: " + e.getMessage());
            return null;
        }
    }

    private static String convertPemClientCertToPkcs12(String certPath, String keyPath) {
        try {
            CertificateFactory cf = CertificateFactory.getInstance("X.509");
            X509Certificate clientCert;
            try (InputStream is = new FileInputStream(certPath)) {
                clientCert = (X509Certificate) cf.generateCertificate(is);
            }
            PrivateKey privateKey = loadPrivateKeyFromPem(keyPath);
            KeyStore ks = KeyStore.getInstance("PKCS12");
            ks.load(null, null);
            ks.setKeyEntry("client-key", privateKey, KEYSTORE_PASSWORD.toCharArray(),
                    new Certificate[]{clientCert});
            Path tempFile = Files.createTempFile("sqlkit-client-", ".p12");
            try (OutputStream os = Files.newOutputStream(tempFile)) {
                ks.store(os, KEYSTORE_PASSWORD.toCharArray());
            }
            tempFile.toFile().deleteOnExit();
            return tempFile.toAbsolutePath().toString();
        } catch (Exception e) {
            System.err.println("Failed to convert client cert to PKCS12: " + e.getMessage());
            return null;
        }
    }

    private static PrivateKey loadPrivateKeyFromPem(String keyPath) throws Exception {
        String content = new String(Files.readAllBytes(Path.of(keyPath)));
        boolean isPkcs1 = content.contains("-----BEGIN RSA PRIVATE KEY-----");
        boolean isEc = content.contains("-----BEGIN EC PRIVATE KEY-----");
        content = content.replace("-----BEGIN PRIVATE KEY-----", "")
                         .replace("-----END PRIVATE KEY-----", "")
                         .replace("-----BEGIN RSA PRIVATE KEY-----", "")
                         .replace("-----END RSA PRIVATE KEY-----", "")
                         .replace("-----BEGIN EC PRIVATE KEY-----", "")
                         .replace("-----END EC PRIVATE KEY-----", "")
                         .replaceAll("\\s", "");
        byte[] keyBytes = Base64.getDecoder().decode(content);

        if (isPkcs1) {
            byte[] pkcs8Bytes = wrapRsaPkcs1ToPkcs8(keyBytes);
            PKCS8EncodedKeySpec spec = new PKCS8EncodedKeySpec(pkcs8Bytes);
            return java.security.KeyFactory.getInstance("RSA").generatePrivate(spec);
        }

        if (isEc) {
            byte[] pkcs8Bytes = wrapEcSec1ToPkcs8(keyBytes);
            PKCS8EncodedKeySpec spec = new PKCS8EncodedKeySpec(pkcs8Bytes);
            return java.security.KeyFactory.getInstance("EC").generatePrivate(spec);
        }

        PKCS8EncodedKeySpec spec = new PKCS8EncodedKeySpec(keyBytes);
        try {
            return java.security.KeyFactory.getInstance("RSA").generatePrivate(spec);
        } catch (Exception e) {
            return java.security.KeyFactory.getInstance("EC").generatePrivate(spec);
        }
    }

    private static byte[] wrapRsaPkcs1ToPkcs8(byte[] pkcs1Bytes) {
        byte[] rsaOid = {0x2a, (byte)0x86, 0x48, (byte)0x86, (byte)0xf7, 0x0d, 0x01, 0x01, 0x01};
        byte[] algId = derSequence(derOid(rsaOid), derNull());
        byte[] octetString = derOctetString(pkcs1Bytes);
        return derSequence(derInteger((byte)0), algId, octetString);
    }

    private static byte[] wrapEcSec1ToPkcs8(byte[] sec1Bytes) {
        byte[] ecPubkeyOid = {0x2a, (byte)0x86, 0x48, (byte)0xce, 0x3d, 0x02, 0x01};
        byte[] curveOid = extractCurveOidFromSec1(sec1Bytes);
        if (curveOid == null) {
            int keyLen = estimateEcKeySize(sec1Bytes);
            if (keyLen == 48) {
                curveOid = new byte[]{0x2b, (byte)0x81, 0x04, 0x00, 0x22};
            } else if (keyLen == 66) {
                curveOid = new byte[]{0x2b, (byte)0x81, 0x04, 0x00, 0x23};
            } else {
                curveOid = new byte[]{0x2a, (byte)0x86, 0x48, (byte)0xce, 0x3d, 0x03, 0x01, 0x07};
            }
        }
        byte[] algId = derSequence(derOid(ecPubkeyOid), derOid(curveOid));
        byte[] octetString = derOctetString(sec1Bytes);
        return derSequence(derInteger((byte)0), algId, octetString);
    }

    private static byte[] extractCurveOidFromSec1(byte[] sec1Der) {
        try {
            int idx = 0;
            if (sec1Der[idx] != 0x30) return null;
            idx++;
            idx += derLenSize(sec1Der, idx);
            if (sec1Der[idx] != 0x02) return null;
            idx++;
            int verLen = sec1Der[idx];
            idx += 1 + verLen;
            if (sec1Der[idx] != 0x04) return null;
            idx++;
            int pkLen = sec1Der[idx];
            if (pkLen < 0x80) { idx += 1 + pkLen; }
            else { idx += 2 + ((pkLen & 0x7f) << 8 | sec1Der[idx+1] & 0xff) - (pkLen & 0x7f); }
            if (idx >= sec1Der.length) return null;
            if (sec1Der[idx] == (byte)0xa0) {
                idx++;
                int ctxLen = sec1Der[idx];
                idx++;
                if (sec1Der[idx] == 0x06) {
                    idx++;
                    int oidLen = sec1Der[idx];
                    idx++;
                    byte[] oid = new byte[oidLen];
                    System.arraycopy(sec1Der, idx, oid, 0, oidLen);
                    return oid;
                }
            }
            return null;
        } catch (Exception e) {
            return null;
        }
    }

    private static int estimateEcKeySize(byte[] sec1Der) {
        try {
            int idx = 0;
            if (sec1Der[idx] != 0x30) return 32;
            idx++;
            idx += derLenSize(sec1Der, idx);
            if (sec1Der[idx] != 0x02) return 32;
            idx++;
            int verLen = sec1Der[idx];
            idx += 1 + verLen;
            if (sec1Der[idx] != 0x04) return 32;
            idx++;
            return sec1Der[idx];
        } catch (Exception e) {
            return 32;
        }
    }

    private static int derLenSize(byte[] der, int idx) {
        if ((der[idx] & 0xff) < 0x80) return 1;
        return 1 + (der[idx] & 0x7f);
    }

    private static byte[] derLength(int length) {
        if (length < 0x80) {
            return new byte[]{(byte) length};
        } else if (length < 0x100) {
            return new byte[]{(byte) 0x81, (byte) length};
        } else {
            return new byte[]{(byte) 0x82, (byte) (length >> 8), (byte) length};
        }
    }

    private static byte[] derSequence(byte[]... elements) {
        java.io.ByteArrayOutputStream out = new java.io.ByteArrayOutputStream();
        int contentLen = 0;
        for (byte[] e : elements) contentLen += e.length;
        byte[] lenBytes = derLength(contentLen);
        out.write(0x30);
        out.write(lenBytes, 0, lenBytes.length);
        for (byte[] e : elements) out.write(e, 0, e.length);
        return out.toByteArray();
    }

    private static byte[] derInteger(byte value) {
        return new byte[]{0x02, 0x01, value};
    }

    private static byte[] derNull() {
        return new byte[]{0x05, 0x00};
    }

    private static byte[] derOid(byte[] oidBytes) {
        byte[] lenBytes = derLength(oidBytes.length);
        java.io.ByteArrayOutputStream out = new java.io.ByteArrayOutputStream();
        out.write(0x06);
        out.write(lenBytes, 0, lenBytes.length);
        out.write(oidBytes, 0, oidBytes.length);
        return out.toByteArray();
    }

    private static byte[] derOctetString(byte[] content) {
        byte[] lenBytes = derLength(content.length);
        java.io.ByteArrayOutputStream out = new java.io.ByteArrayOutputStream();
        out.write(0x04);
        out.write(lenBytes, 0, lenBytes.length);
        out.write(content, 0, content.length);
        return out.toByteArray();
    }

    private static String mapToPostgresSslMode(String sslMode) {
        switch (sslMode) {
            case "verify-ca": return "verify-ca";
            case "verify-full": return "verify-full";
            case "require": return "require";
            case "prefer": return "prefer";
            default: return "disable";
        }
    }

    private static String mapToMysqlSslMode(String sslMode) {
        switch (sslMode) {
            case "verify-full": return "VERIFY_IDENTITY";
            case "verify-ca": return "VERIFY_CA";
            case "require": return "REQUIRED";
            case "prefer": return "PREFERRED";
            default: return "DISABLED";
        }
    }
}
