package io.github.juanluci.starling;

import java.io.File;
import java.io.FileOutputStream;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.util.Locale;
import org.json.JSONArray;
import org.json.JSONException;
import org.json.JSONObject;

/**
 * Turns an Amethyst / PojavLauncher control file (`mControlDataList`, buttons placed by
 * `${screen_width}` expressions, GLFW key codes) into a Starling GoyControl preset, stored in
 * `controls/goycontrol/<name>.json` of the game folder, where the in-game preset selector finds
 * it. Nothing here touches Android's UI: it reads and writes files only.
 *
 * <p>Buttons keep their position, size, opacity, colours, border, toggle, swipe and pass-through.
 * A button whose keys have no Starling counterpart (the on-screen keyboard, hide-controls, scroll
 * wheel and menu buttons) is left out and counted in {@link Result#skipped}.
 */
final class PojavControls {
    /** What an import did. */
    static final class Result {
        final String name;
        final int imported;
        final int skipped;
        final boolean joystick;

        Result(String name, int imported, int skipped, boolean joystick) {
            this.name = name;
            this.imported = imported;
            this.skipped = skipped;
            this.joystick = joystick;
        }
    }

    /** Pojav's expression variable `${margin}`, in its units. */
    private static final double MARGIN = 2.0;
    /** Pojav's default button scale preference, `${preferred_scale}`. */
    private static final double PREFERRED_SCALE = 100.0;

    private PojavControls() {}

    /**
     * Convert `json` and write the preset. `screenW`/`screenH` are the phone's landscape size in
     * dp (the long side first). `gameFolder` is the game folder; `fileName` names the preset.
     */
    static Result importInto(File gameFolder, String json, String fileName, double screenW,
            double screenH) throws JSONException, IOException {
        String name = presetName(fileName);
        JSONObject source = new JSONObject(json);
        JSONArray list = source.optJSONArray("mControlDataList");
        if (list == null) {
            throw new JSONException("not an Amethyst/Pojav control file (no mControlDataList)");
        }
        JSONObject preset = new JSONObject();
        preset.put("control_style", "goy-control");
        preset.put("split_controls", true);
        preset.put("preset_id", name);
        preset.put("preset_name", name);
        preset.put("preset_description", "Imported from an Amethyst/Pojav control file");
        preset.put("active_profile", name);
        preset.put("active_goy_preset", name);

        JSONArray buttons = new JSONArray();
        int skipped = 0;
        for (int i = 0; i < list.length(); i++) {
            JSONObject data = list.getJSONObject(i);
            JSONObject button = button(data, screenW, screenH);
            if (button == null) {
                skipped++;
            } else {
                buttons.put(button);
            }
        }
        preset.put("custom_buttons", buttons);

        JSONArray sticks = source.optJSONArray("mJoystickDataList");
        boolean joystick = false;
        File touch = new File(gameFolder, "horizon-touch.json");
        if (sticks != null && sticks.length() > 0) {
            joystick = placeJoystick(preset, touch, sticks.getJSONObject(0), screenW, screenH);
        }
        write(new File(gameFolder, "controls/goycontrol/" + name + ".json"), preset);
        select(touch, name, buttons);
        return new Result(name, buttons.length(), skipped, joystick);
    }

    /** A preset file name: Starling's own rule for profile names (a-z, 0-9, - and _). */
    static String presetName(String fileName) {
        String base = fileName == null ? "" : fileName;
        int dot = base.lastIndexOf('.');
        if (dot > 0) {
            base = base.substring(0, dot);
        }
        StringBuilder out = new StringBuilder();
        for (char c : base.trim().toLowerCase(Locale.ROOT).toCharArray()) {
            out.append((c >= 'a' && c <= 'z') || (c >= '0' && c <= '9') || c == '-' || c == '_'
                    ? c : '-');
        }
        String name = out.toString();
        return name.isEmpty() || name.equals("default") ? "amethyst-import" : name;
    }

    /** One Starling custom button, or null when its keys do not exist in Starling. */
    private static JSONObject button(JSONObject data, double screenW, double screenH)
            throws JSONException {
        if (!data.optBoolean("displayInGame", true) && !data.optBoolean("displayInMenu", true)) {
            return null;
        }
        JSONArray codes = data.optJSONArray("keycodes");
        String[] keys = new String[3];
        int count = 0;
        for (int i = 0; codes != null && i < codes.length() && count < 3; i++) {
            String key = keyName(codes.optInt(i, 0));
            if (key != null) {
                keys[count++] = key;
            }
        }
        if (count == 0) {
            return null;
        }
        double w = data.optDouble("width", 50);
        double h = data.optDouble("height", 50);
        double x = eval(data.optString("dynamicX", "0"), screenW, screenH, w, h);
        double y = eval(data.optString("dynamicY", "0"), screenW, screenH, w, h);

        JSONObject layout = new JSONObject();
        layout.put("center", new JSONArray().put(clamp((x + w / 2) / screenW))
                .put(clamp((y + h / 2) / screenH)));
        layout.put("size", round(h / screenH));
        layout.put("opacity", round(data.optDouble("opacity", 1.0)));
        layout.put("passthrough", data.optBoolean("passThruEnabled", false));
        layout.put("toggleable", data.optBoolean("isToggle", false));
        layout.put("swipeable", data.optBoolean("isSwipeable", false));
        layout.put("shape", 0);
        layout.put("icon_id", 0);
        layout.put("width_ratio", round(w / h));
        layout.put("height_ratio", 1.0);
        layout.put("use_goy_style", true);
        layout.put("stroke_size", round(Math.max(data.optDouble("strokeWidth", 1.0), 0.0)));
        layout.put("stroke_margin", 0.0);
        layout.put("stroke_roundness", round(data.optDouble("cornerRadius", 0.0) + 12.0));
        layout.put("stroke_color", unsigned(data.optInt("strokeColor", -1)));
        layout.put("button_color", unsigned(data.optInt("bgColor", 0)));
        layout.put("shadow_size", 0.0);
        layout.put("shadow_color", 2516582400L);
        layout.put("roundness", round(data.optDouble("cornerRadius", 0.0) + 12.0));
        layout.put("on_click_color", 4282529397L);
        layout.put("on_click_stroke_color", 4291869951L);
        layout.put("on_toggle_color", 4283710356L);
        layout.put("on_toggle_stroke_color", 4293582335L);
        layout.put("visible_in_game", data.optBoolean("displayInGame", true));
        layout.put("visible_in_menu", data.optBoolean("displayInMenu", true));

        JSONObject button = new JSONObject();
        button.put("key", keys[0]);
        button.put("second_key", keys[1] == null ? "" : keys[1]);
        button.put("third_key", keys[2] == null ? "" : keys[2]);
        String label = data.optString("name", "").trim().toUpperCase(Locale.ROOT);
        button.put("custom_label", label.length() > 8 ? label.substring(0, 8) : label);
        button.put("always_visible", false);
        button.put("layouts", new JSONArray().put(layout).put(new JSONObject(layout.toString()))
                .put(new JSONObject(layout.toString())));
        return button;
    }

    /**
     * Put the move stick where Pojav had it, in the three layouts of the player's current
     * settings (their other controls stay as they are). Without a settings file there is nothing
     * to edit, so Starling's own stick position stays.
     */
    private static boolean placeJoystick(JSONObject preset, File touch, JSONObject stick,
            double screenW, double screenH) {
        try {
            if (!touch.isFile()) {
                return false;
            }
            JSONObject settings = new JSONObject(
                    new String(java.nio.file.Files.readAllBytes(touch.toPath()),
                            StandardCharsets.UTF_8));
            JSONArray layouts = settings.optJSONArray("layouts");
            if (layouts == null || layouts.length() == 0) {
                return false;
            }
            double w = stick.optDouble("width", 130);
            double h = stick.optDouble("height", 130);
            double x = eval(stick.optString("dynamicX", "0"), screenW, screenH, w, h);
            double y = eval(stick.optString("dynamicY", "0"), screenW, screenH, w, h);
            for (int i = 0; i < layouts.length(); i++) {
                JSONArray one = layouts.getJSONArray(i);
                JSONObject move = one.getJSONObject(0);
                move.put("center", new JSONArray().put(clamp((x + w / 2) / screenW))
                        .put(clamp((y + h / 2) / screenH)));
                move.put("size", round(h / screenH));
            }
            preset.put("layouts", layouts);
            return true;
        } catch (Exception e) {
            return false;
        }
    }

    /** Make the imported preset the active GoyControl one, with its buttons, in the settings. */
    private static void select(File touch, String name, JSONArray buttons) {
        try {
            JSONObject settings = touch.isFile()
                    ? new JSONObject(new String(java.nio.file.Files.readAllBytes(touch.toPath()),
                            StandardCharsets.UTF_8))
                    : new JSONObject();
            settings.put("control_style", "goy-control");
            settings.put("active_goy_preset", name);
            settings.put("active_profile", name);
            // The settings file holds the active layout's buttons as well as the preset does.
            settings.put("custom_buttons", new JSONArray(buttons.toString()));
            write(touch, settings);
        } catch (Exception e) {
            android.util.Log.w("starling", "could not select the imported preset", e);
        }
    }

    private static void write(File file, JSONObject json) throws IOException {
        File parent = file.getParentFile();
        if (parent != null) {
            parent.mkdirs();
        }
        File temp = new File(file.getPath() + ".tmp");
        try (FileOutputStream out = new FileOutputStream(temp)) {
            out.write(json.toString(2).getBytes(StandardCharsets.UTF_8));
        } catch (JSONException e) {
            throw new IOException(e);
        }
        if (!temp.renameTo(file)) {
            file.delete();
            if (!temp.renameTo(file)) {
                throw new IOException("could not write " + file);
            }
        }
    }

    /** Starling's key name for a GLFW key code or Pojav special code, or null. */
    static String keyName(int code) {
        switch (code) {
            case -3: return "key.mouse.left";
            case -4: return "key.mouse.right";
            case -5: return "mouse.virtual.toggle";
            case -6: return "key.mouse.middle";
            case 32: return "key.keyboard.space";
            case 39: return "key.keyboard.apostrophe";
            case 44: return "key.keyboard.comma";
            case 45: return "key.keyboard.minus";
            case 46: return "key.keyboard.period";
            case 47: return "key.keyboard.slash";
            case 59: return "key.keyboard.semicolon";
            case 61: return "key.keyboard.equal";
            case 91: return "key.keyboard.left.bracket";
            case 92: return "key.keyboard.backslash";
            case 93: return "key.keyboard.right.bracket";
            case 96: return "key.keyboard.grave.accent";
            case 256: return "key.keyboard.escape";
            case 257: return "key.keyboard.enter";
            case 258: return "key.keyboard.tab";
            case 259: return "key.keyboard.backspace";
            case 260: return "key.keyboard.insert";
            case 261: return "key.keyboard.delete";
            case 262: return "key.keyboard.right";
            case 263: return "key.keyboard.left";
            case 264: return "key.keyboard.down";
            case 265: return "key.keyboard.up";
            case 266: return "key.keyboard.page.up";
            case 267: return "key.keyboard.page.down";
            case 268: return "key.keyboard.home";
            case 269: return "key.keyboard.end";
            case 280: return "key.keyboard.caps.lock";
            case 340: return "key.keyboard.left.shift";
            case 341: return "key.keyboard.left.control";
            case 342: return "key.keyboard.left.alt";
            case 344: return "key.keyboard.right.shift";
            case 345: return "key.keyboard.right.control";
            case 346: return "key.keyboard.right.alt";
            default:
                break;
        }
        if (code >= 48 && code <= 57) {
            return "key.keyboard." + (char) code;
        }
        if (code >= 65 && code <= 90) {
            return "key.keyboard." + (char) (code + 32);
        }
        if (code >= 290 && code <= 301) {
            return "key.keyboard.f" + (code - 289);
        }
        if (code >= 320 && code <= 329) {
            return "key.keyboard.keypad." + (code - 320);
        }
        return null;
    }

    /** Pojav's position expression: numbers, + - * /, parentheses, `${var}` and `px(n)`. */
    static double eval(String expression, double screenW, double screenH, double w, double h) {
        try {
            return new Parser(expression, screenW, screenH, w, h).parse();
        } catch (RuntimeException e) {
            return 0.0;
        }
    }

    private static final class Parser {
        private final String s;
        private final double screenW, screenH, w, h;
        private int i;

        Parser(String s, double screenW, double screenH, double w, double h) {
            this.s = s;
            this.screenW = screenW;
            this.screenH = screenH;
            this.w = w;
            this.h = h;
        }

        double parse() {
            double v = sum();
            skip();
            if (i != s.length()) {
                throw new IllegalArgumentException("trailing text");
            }
            return v;
        }

        private void skip() {
            while (i < s.length() && s.charAt(i) == ' ') {
                i++;
            }
        }

        private double sum() {
            double v = product();
            for (;;) {
                skip();
                if (i < s.length() && s.charAt(i) == '+') {
                    i++;
                    v += product();
                } else if (i < s.length() && s.charAt(i) == '-') {
                    i++;
                    v -= product();
                } else {
                    return v;
                }
            }
        }

        private double product() {
            double v = unary();
            for (;;) {
                skip();
                if (i < s.length() && s.charAt(i) == '*') {
                    i++;
                    v *= unary();
                } else if (i < s.length() && s.charAt(i) == '/') {
                    i++;
                    v /= unary();
                } else {
                    return v;
                }
            }
        }

        private double unary() {
            skip();
            if (i < s.length() && s.charAt(i) == '-') {
                i++;
                return -unary();
            }
            return atom();
        }

        private double atom() {
            skip();
            if (i >= s.length()) {
                throw new IllegalArgumentException("unexpected end");
            }
            char c = s.charAt(i);
            if (c == '(') {
                i++;
                double v = sum();
                skip();
                i++;
                return v;
            }
            if (s.startsWith("px(", i)) {
                i += 3;
                double v = sum();
                skip();
                i++;
                return v;
            }
            if (s.startsWith("${", i)) {
                int end = s.indexOf('}', i);
                String name = s.substring(i + 2, end);
                i = end + 1;
                switch (name) {
                    case "screen_width": return screenW;
                    case "screen_height": return screenH;
                    case "width": return w;
                    case "height": return h;
                    case "margin": return MARGIN;
                    case "preferred_scale": return PREFERRED_SCALE;
                    default: throw new IllegalArgumentException(name);
                }
            }
            int start = i;
            while (i < s.length() && (Character.isDigit(s.charAt(i)) || s.charAt(i) == '.'
                    || ((s.charAt(i) == 'E' || s.charAt(i) == 'e') && i > start)
                    || ((s.charAt(i) == '-' || s.charAt(i) == '+') && i > start
                            && (s.charAt(i - 1) == 'E' || s.charAt(i - 1) == 'e')))) {
                i++;
            }
            return Double.parseDouble(s.substring(start, i));
        }
    }

    private static double clamp(double v) {
        return Math.max(0.0, Math.min(1.0, round(v)));
    }

    private static double round(double v) {
        return Math.round(v * 10000.0) / 10000.0;
    }

    /** Android colours are signed ints; Starling stores them as unsigned 32-bit numbers. */
    private static long unsigned(int color) {
        return color & 0xFFFFFFFFL;
    }
}
