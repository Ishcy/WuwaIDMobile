package frb.axeron.server;

interface IOutputStreamCallback {
    void onComplete();
    void onError(int errCode, String message);
}
