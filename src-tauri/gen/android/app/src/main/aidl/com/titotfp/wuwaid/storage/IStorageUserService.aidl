package com.titotfp.wuwaid.storage;

import android.os.Bundle;
import android.os.ParcelFileDescriptor;

interface IStorageUserService {
    void destroy() = 16777114;
    Bundle execute(String operation, String first, String second, in ParcelFileDescriptor input, long requestId) = 0;
    Bundle metadata(String operation, String first, String second, long requestId) = 1;
}
