package frb.axeron.server;

interface IAxeronService {
    IFileService getFileService() = 2;
    void requestPermission(int requestCode) = 19;
    boolean checkSelfPermission() = 20;
    boolean shouldShowRequestPermissionRationale() = 21;
}
