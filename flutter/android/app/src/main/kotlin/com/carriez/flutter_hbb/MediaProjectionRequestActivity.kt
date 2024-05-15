package com.carriez.flutter_hbb


import android.content.Intent
import android.media.projection.MediaProjectionManager
import android.os.Build
import android.os.Bundle
import android.util.Log
import androidx.appcompat.app.AppCompatActivity
import com.carriez.flutter_hbb.MainService


class MediaProjectionRequestActivity : AppCompatActivity() {

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val mMediaProjectionManager =
            getSystemService(MEDIA_PROJECTION_SERVICE) as MediaProjectionManager
        // ask for MediaProjection right away
        Log.i(TAG, "Requesting confirmation")
        // This initiates a prompt dialog for the user to confirm screen projection.
        startActivityForResult(
            mMediaProjectionManager.createScreenCaptureIntent(),
            REQUEST_MEDIA_PROJECTION
        )

    }

    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        super.onActivityResult(requestCode, resultCode, data)
        if (requestCode == REQUEST_MEDIA_PROJECTION) {
            if (resultCode != RESULT_OK) Log.i(TAG, "User cancelled") else Log.i(
                TAG, "User acknowledged"
            )
            val intent = Intent(this, MainService::class.java)
            intent.setAction(ACTION_HANDLE_MEDIA_PROJECTION_RESULT)
            intent.putExtra(EXTRA_MEDIA_PROJECTION_RESULT_CODE, resultCode)
            intent.putExtra(EXTRA_MEDIA_PROJECTION_RESULT_DATA, data)
            startService(intent)
            finish()
        }
    }

    companion object {
        private const val TAG = "MPRequestActivity"
        private const val REQUEST_MEDIA_PROJECTION = 42
        const val EXTRA_UPGRADING_FROM_FALLBACK_SCREEN_CAPTURE =
            "upgrading_from_fallback_screen_capture"
    }
}