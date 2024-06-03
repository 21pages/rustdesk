package com.carriez.flutter_hbb

import android.app.PendingIntent
import android.app.Service
import android.content.Intent
import android.graphics.Color
import android.graphics.PixelFormat
import android.graphics.Point
import android.graphics.drawable.Drawable
import android.os.Build
import android.os.IBinder
import android.view.Display
import android.view.Gravity
import android.view.LayoutInflater
import android.view.View
import android.view.WindowManager
import android.widget.ImageButton
import android.widget.ImageView
import android.widget.PopupMenu
import android.widget.TextView

class FloatingWindowService : Service() {

    private lateinit var windowManager: WindowManager
    private lateinit var params: WindowManager.LayoutParams
    private lateinit var floatingView: ImageView

    override fun onBind(intent: Intent): IBinder? {
        return null
    }

    override fun onCreate() {
        super.onCreate()

        floatingView = ImageView(this)

        val drawable: Drawable = resources.getDrawable(R.mipmap.ic_launcher_floating, null)
        // drawable.alpha = 255
        floatingView.setImageDrawable(drawable)

        floatingView.setOnClickListener { view ->
            // showPopupMenu(view)
            val intent = Intent(this, MainActivity::class.java)
            intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
            val pendingIntent = PendingIntent.getActivity(this, 0, intent,
                PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_ONE_SHOT)
            try {
                pendingIntent.send()
            } catch (e: PendingIntent.CanceledException) {
                e.printStackTrace()
            }
        }

        params = WindowManager.LayoutParams(
//            WindowManager.LayoutParams.WRAP_CONTENT,
//            WindowManager.LayoutParams.WRAP_CONTENT,
            100,
            100,
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) WindowManager.LayoutParams.TYPE_APPLICATION_OVERLAY else WindowManager.LayoutParams.TYPE_PHONE,
            WindowManager.LayoutParams.FLAG_NOT_FOCUSABLE,
            PixelFormat.TRANSLUCENT
        )

        params.gravity = Gravity.TOP or Gravity.START
        params.x = -30
        params.y = 100

        windowManager = getSystemService(WINDOW_SERVICE) as WindowManager

        windowManager.addView(floatingView, params)
    }

    override fun onDestroy() {
        super.onDestroy()
        windowManager.removeView(floatingView)
    }

    // private fun showPopupMenu(view: View) {
    //     // Create a PopupMenu, assign it a Menu XML file, and show it
    //     val popupMenu = PopupMenu(this, view)
    //     popupMenu.menu.add(0, 0, 0, "close connection")

    //     popupMenu.setOnMenuItemClickListener { menuItem ->
    //         when (menuItem.itemId) {
    //             0 -> {
    //                 // Handle the "Close Connection" menu item click here
    //                 true
    //             }
    //             else -> false
    //         }
    //     }
    //     popupMenu.show()
    // }
}
