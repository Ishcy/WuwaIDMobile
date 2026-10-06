package frb.axeron.server;

import android.os.ParcelFileDescriptor;
import frb.axeron.server.IOutputStreamCallback;

interface IFileService {
    boolean mkdirs(String dirPath);
    boolean createNewFile(String path);
    boolean delete(String path);
    boolean exists(String path);
    boolean fsync(in ParcelFileDescriptor pfd);
    FileStat stat(String path);
    String getCanonicalPath(String path);
    List<FileStat> list(String dirPath, int filter, int sortBy, boolean ascending, int offset, int limit);
    boolean setLastModified(String path, long newLastModified);
    boolean chmod(String path, int mode);
    boolean chown(String path, int uid, int gid);
    ParcelFileDescriptor inputStreamPfd(String path);
    void outputStreamPfd(String path, in ParcelFileDescriptor inputStreamPfd, in IOutputStreamCallback callback, boolean append);
    boolean renameTo(String src, String dst, boolean overwrite);
}
