package io.github.juanluci.starling;

import android.app.Activity;
import android.app.AlertDialog;
import android.content.Intent;
import android.content.pm.ActivityInfo;
import android.database.Cursor;
import android.graphics.Color;
import android.graphics.Typeface;
import android.graphics.drawable.GradientDrawable;
import android.net.Uri;
import android.os.Bundle;
import android.provider.DocumentsContract;
import android.provider.OpenableColumns;
import android.util.DisplayMetrics;
import android.util.TypedValue;
import android.view.Gravity;
import android.view.View;
import android.view.ViewGroup;
import android.widget.Button;
import android.widget.CompoundButton;
import android.widget.LinearLayout;
import android.widget.ScrollView;
import android.widget.SeekBar;
import android.widget.Switch;
import android.widget.TextView;
import android.widget.Toast;
import java.io.ByteArrayOutputStream;
import java.io.File;
import java.io.InputStream;
import java.nio.charset.StandardCharsets;
import org.json.JSONObject;

/**
 * The portrait home screen, laid out as Amethyst's launcher is: an account bar with a settings
 * button on top, a list of menu buttons, and a bottom bar with the version and a wide PLAY. PLAY
 * starts the game (GameActivity), which is landscape as before. The controls page edits the look
 * sensitivity in the player's `horizon-touch.json`, and imports Amethyst/Pojav control files
 * (PojavControls). Built in code: Starling's APK has no layout resources.
 */
public final class HomeActivity extends Activity {
    private static final int PICK_CONTROLS = 11;
    private static final int PICK_FILE = 12;

    private static final int BACKGROUND = 0xFF181818;
    private static final int BAR = 0xFF232323;
    private static final int BUTTON = 0xFF2C2C2C;
    private static final int TEXT = 0xFFFFFFFF;
    private static final int MUTED = 0xFFB0B0B0;
    private static final int PLAY = 0xFF3C8527;

    private File gameFolder;

    @Override
    @SuppressWarnings("deprecation")
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        try {
            setRequestedOrientation(ActivityInfo.SCREEN_ORIENTATION_PORTRAIT);
        } catch (IllegalStateException e) {
            // As it is, then.
        }
        gameFolder = GameFolder.choose(getFilesDir(), StarlingActivity.sharedFolder(this)).root();
        showHome();
    }

    private int dp(float value) {
        return Math.round(TypedValue.applyDimension(TypedValue.COMPLEX_UNIT_DIP, value,
                getResources().getDisplayMetrics()));
    }

    private static GradientDrawable box(int color, float radius) {
        GradientDrawable shape = new GradientDrawable();
        shape.setColor(color);
        shape.setCornerRadius(radius);
        return shape;
    }

    private TextView label(String text, float sp, int color, boolean bold) {
        TextView view = new TextView(this);
        view.setText(text);
        view.setTextSize(sp);
        view.setTextColor(color);
        if (bold) {
            view.setTypeface(Typeface.DEFAULT, Typeface.BOLD);
        }
        return view;
    }

    /** An Amethyst menu button: a full-width dark row with a label. */
    private Button menuButton(String text, View.OnClickListener click) {
        Button button = new Button(this);
        button.setText(text);
        button.setAllCaps(false);
        button.setTextColor(TEXT);
        button.setTextSize(15);
        button.setGravity(Gravity.CENTER_VERTICAL | Gravity.START);
        button.setPadding(dp(20), 0, dp(20), 0);
        button.setBackground(box(BUTTON, dp(4)));
        LinearLayout.LayoutParams params = new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, dp(52));
        params.setMargins(dp(12), dp(6), dp(12), 0);
        button.setLayoutParams(params);
        button.setOnClickListener(click);
        return button;
    }

    private void showHome() {
        LinearLayout root = new LinearLayout(this);
        root.setOrientation(LinearLayout.VERTICAL);
        root.setBackgroundColor(BACKGROUND);
        root.setFitsSystemWindows(true);

        // Top bar: the account on the left (accounts are managed in the game), settings right.
        LinearLayout top = new LinearLayout(this);
        top.setOrientation(LinearLayout.HORIZONTAL);
        top.setGravity(Gravity.CENTER_VERTICAL);
        top.setBackgroundColor(BAR);
        top.setPadding(dp(16), 0, dp(4), 0);
        LinearLayout account = new LinearLayout(this);
        account.setOrientation(LinearLayout.VERTICAL);
        account.addView(label("Starling", 17, TEXT, true));
        account.addView(label("Sign in and accounts: inside the game", 12, MUTED, false));
        top.addView(account, new LinearLayout.LayoutParams(0, dp(64), 1f));
        Button settings = new Button(this);
        settings.setText("⚙");
        settings.setTextSize(24);
        settings.setTextColor(TEXT);
        settings.setBackgroundColor(Color.TRANSPARENT);
        settings.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View v) {
                showControls();
            }
        });
        top.addView(settings, new LinearLayout.LayoutParams(dp(56), dp(56)));
        root.addView(top, new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT));

        // The menu.
        ScrollView scroll = new ScrollView(this);
        LinearLayout menu = new LinearLayout(this);
        menu.setOrientation(LinearLayout.VERTICAL);
        menu.setPadding(0, dp(6), 0, dp(12));
        menu.addView(menuButton("Controls and sensitivity", new View.OnClickListener() {
            @Override
            public void onClick(View v) {
                showControls();
            }
        }));
        menu.addView(menuButton("Import Amethyst control file", new View.OnClickListener() {
            @Override
            public void onClick(View v) {
                pick(PICK_CONTROLS);
            }
        }));
        menu.addView(menuButton("Import a pack, world or mod file", new View.OnClickListener() {
            @Override
            public void onClick(View v) {
                pick(PICK_FILE);
            }
        }));
        menu.addView(menuButton("Open game folder", new View.OnClickListener() {
            @Override
            public void onClick(View v) {
                openFolder();
            }
        }));
        menu.addView(menuButton("Diagnostics and logs", new View.OnClickListener() {
            @Override
            public void onClick(View v) {
                startActivity(new Intent(HomeActivity.this, DiagnosticsActivity.class));
            }
        }));
        scroll.addView(menu);
        root.addView(scroll, new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, 0, 1f));

        // Bottom bar: version, then PLAY.
        LinearLayout bottom = new LinearLayout(this);
        bottom.setOrientation(LinearLayout.VERTICAL);
        bottom.setBackgroundColor(BAR);
        bottom.setPadding(dp(17), dp(10), dp(17), dp(15));
        bottom.addView(label("Starling " + versionName(), 14, MUTED, false));
        Button play = new Button(this);
        play.setText("PLAY");
        play.setTextSize(18);
        play.setTypeface(Typeface.DEFAULT, Typeface.BOLD);
        play.setTextColor(TEXT);
        play.setBackground(box(PLAY, dp(4)));
        play.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View v) {
                play();
            }
        });
        LinearLayout.LayoutParams playParams = new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, dp(62));
        playParams.topMargin = dp(8);
        bottom.addView(play, playParams);
        root.addView(bottom, new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT));

        setContentView(root);
    }

    /** The controls page: look sensitivity, inversion, handedness and the control import. */
    private void showControls() {
        final JSONObject touch = readTouch();
        LinearLayout root = new LinearLayout(this);
        root.setOrientation(LinearLayout.VERTICAL);
        root.setBackgroundColor(BACKGROUND);
        root.setFitsSystemWindows(true);

        TextView title = label("Controls", 18, TEXT, true);
        title.setBackgroundColor(BAR);
        title.setPadding(dp(16), dp(18), dp(16), dp(18));
        root.addView(title, new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT));

        ScrollView scroll = new ScrollView(this);
        LinearLayout page = new LinearLayout(this);
        page.setOrientation(LinearLayout.VERTICAL);
        page.setPadding(dp(16), dp(8), dp(16), dp(16));
        page.addView(slider(touch, "sensitivity", "Look sensitivity"));
        page.addView(slider(touch, "sensitivity_horizontal", "Horizontal sensitivity"));
        page.addView(slider(touch, "sensitivity_vertical", "Vertical sensitivity"));
        page.addView(toggle(touch, "invert_y", "Invert Y axis"));
        page.addView(toggle(touch, "left_handed", "Left-handed layout"));
        page.addView(toggle(touch, "auto_jump", "Auto jump"));
        page.addView(toggle(touch, "attack_button_look", "Look while holding attack"));
        page.addView(menuButton("Import Amethyst control file", new View.OnClickListener() {
            @Override
            public void onClick(View v) {
                pick(PICK_CONTROLS);
            }
        }));
        scroll.addView(page);
        root.addView(scroll, new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, 0, 1f));

        Button back = menuButton("Back", new View.OnClickListener() {
            @Override
            public void onClick(View v) {
                writeTouch(touch);
                showHome();
            }
        });
        back.setGravity(Gravity.CENTER);
        root.addView(back);
        setContentView(root);
    }

    private View slider(final JSONObject touch, final String key, String name) {
        LinearLayout row = new LinearLayout(this);
        row.setOrientation(LinearLayout.VERTICAL);
        row.setPadding(0, dp(10), 0, dp(4));
        final TextView text = label(name + ": " + format(touch.optDouble(key, 1.0)), 14, TEXT,
                false);
        row.addView(text);
        SeekBar bar = new SeekBar(this);
        // 0.10x to 3.00x in steps of 0.05.
        bar.setMax(58);
        bar.setProgress((int) Math.round((touch.optDouble(key, 1.0) - 0.1) / 0.05));
        final String title = name;
        bar.setOnSeekBarChangeListener(new SeekBar.OnSeekBarChangeListener() {
            @Override
            public void onProgressChanged(SeekBar seekBar, int progress, boolean user) {
                double value = 0.1 + progress * 0.05;
                try {
                    touch.put(key, Math.round(value * 100.0) / 100.0);
                } catch (org.json.JSONException e) {
                    // Cannot happen for a finite number.
                }
                text.setText(title + ": " + format(value));
            }

            @Override
            public void onStartTrackingTouch(SeekBar seekBar) {}

            @Override
            public void onStopTrackingTouch(SeekBar seekBar) {
                writeTouch(touch);
            }
        });
        row.addView(bar);
        return row;
    }

    private View toggle(final JSONObject touch, final String key, String name) {
        Switch view = new Switch(this);
        view.setText(name);
        view.setTextColor(TEXT);
        view.setTextSize(14);
        view.setPadding(0, dp(12), 0, dp(12));
        view.setChecked(touch.optBoolean(key, false));
        view.setOnCheckedChangeListener(new CompoundButton.OnCheckedChangeListener() {
            @Override
            public void onCheckedChanged(CompoundButton button, boolean checked) {
                try {
                    touch.put(key, checked);
                } catch (org.json.JSONException e) {
                    // Cannot happen for a boolean.
                }
                writeTouch(touch);
            }
        });
        return view;
    }

    private static String format(double value) {
        return String.format(java.util.Locale.ROOT, "%.2fx", value);
    }

    private File touchFile() {
        return new File(gameFolder, "horizon-touch.json");
    }

    private JSONObject readTouch() {
        try {
            File file = touchFile();
            if (file.isFile()) {
                return new JSONObject(new String(java.nio.file.Files.readAllBytes(file.toPath()),
                        StandardCharsets.UTF_8));
            }
        } catch (Exception e) {
            android.util.Log.w("starling", "could not read horizon-touch.json", e);
        }
        return new JSONObject();
    }

    private void writeTouch(JSONObject touch) {
        try {
            gameFolder.mkdirs();
            File temp = new File(gameFolder, "horizon-touch.json.tmp");
            java.nio.file.Files.write(temp.toPath(),
                    touch.toString(2).getBytes(StandardCharsets.UTF_8));
            if (!temp.renameTo(touchFile())) {
                touchFile().delete();
                temp.renameTo(touchFile());
            }
        } catch (Exception e) {
            Toast.makeText(this, "Could not save the settings", Toast.LENGTH_SHORT).show();
        }
    }

    private String versionName() {
        try {
            return getPackageManager().getPackageInfo(getPackageName(), 0).versionName;
        } catch (Exception e) {
            return "";
        }
    }

    private void pick(int request) {
        Intent intent = new Intent(Intent.ACTION_OPEN_DOCUMENT);
        intent.addCategory(Intent.CATEGORY_OPENABLE);
        intent.setType("*/*");
        startActivityForResult(intent, request);
    }

    private void openFolder() {
        Uri uri = DocumentsContract.buildRootUri(getPackageName() + ".documents",
                GameFolder.ROOT);
        Intent intent = new Intent(Intent.ACTION_VIEW);
        intent.setDataAndType(uri, DocumentsContract.Root.MIME_TYPE_ITEM);
        intent.addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION);
        try {
            startActivity(intent);
        } catch (Exception e) {
            Toast.makeText(this, "No file manager can open the game folder",
                    Toast.LENGTH_LONG).show();
        }
    }

    private void play() {
        Intent game = new Intent(this, GameActivity.class);
        game.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
        startActivity(game);
        finish();
        overridePendingTransition(0, 0);
    }

    @Override
    protected void onActivityResult(int request, int result, Intent data) {
        super.onActivityResult(request, result, data);
        if (result != RESULT_OK || data == null || data.getData() == null) {
            return;
        }
        Uri uri = data.getData();
        if (request == PICK_FILE) {
            LauncherActivity.receive(this, uri, new Runnable() {
                @Override
                public void run() {
                    Toast.makeText(HomeActivity.this,
                            "Handed to Starling: it installs when the game starts",
                            Toast.LENGTH_LONG).show();
                }
            });
        } else if (request == PICK_CONTROLS) {
            importControls(uri);
        }
    }

    private void importControls(final Uri uri) {
        final DisplayMetrics metrics = getResources().getDisplayMetrics();
        final double longSide = Math.max(metrics.widthPixels, metrics.heightPixels)
                / metrics.density;
        final double shortSide = Math.min(metrics.widthPixels, metrics.heightPixels)
                / metrics.density;
        final String name = displayName(uri);
        new Thread(new Runnable() {
            @Override
            public void run() {
                String message;
                try (InputStream in = getContentResolver().openInputStream(uri)) {
                    ByteArrayOutputStream bytes = new ByteArrayOutputStream();
                    byte[] buffer = new byte[1 << 14];
                    for (int n; in != null && (n = in.read(buffer)) > 0; ) {
                        bytes.write(buffer, 0, n);
                    }
                    PojavControls.Result r = PojavControls.importInto(gameFolder,
                            new String(bytes.toByteArray(), StandardCharsets.UTF_8), name,
                            longSide, shortSide);
                    message = "Imported " + r.imported + " buttons as '" + r.name + "'"
                            + (r.skipped > 0 ? " (" + r.skipped + " skipped)" : "")
                            + (r.joystick ? ", joystick placed" : "")
                            + ". It is selected for GoyControl.";
                } catch (Exception e) {
                    message = "Not imported: " + e.getMessage();
                }
                final String shown = message;
                runOnUiThread(new Runnable() {
                    @Override
                    public void run() {
                        new AlertDialog.Builder(HomeActivity.this,
                                android.R.style.Theme_DeviceDefault_Dialog_Alert)
                                .setMessage(shown).setPositiveButton("OK", null).show();
                    }
                });
            }
        }, "starling-control-import").start();
    }

    private String displayName(Uri uri) {
        try (Cursor cursor = getContentResolver().query(uri,
                new String[] {OpenableColumns.DISPLAY_NAME}, null, null, null)) {
            if (cursor != null && cursor.moveToFirst() && cursor.getString(0) != null) {
                return cursor.getString(0);
            }
        } catch (Exception e) {
            // The path below.
        }
        String last = uri.getLastPathSegment();
        return last == null ? "amethyst-import" : last;
    }
}
