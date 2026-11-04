// Fills in the names derived from the project name, so a new app needs only its name:
//   app_title        "Filebrary" for filebrary (asked; empty = the project name with each word capitalised)
//   android_app      the crate name without underscores (my_app → myapp): a Kotlin package name may not have them
//   android_package  dev.krifmaz.<android_app>, the Android applicationId and Kotlin package; the Kotlin sources are
//                    in dev/krifmaz/<android_app> (cargo-generate turns a / inside a placeholder into _)
//   image            ghcr.io/<owner>/<project-name>, the Docker image
let name = variable::get("project-name");
let title = variable::get("app_title");
if title == "" {
    let words = [];
    for w in name.split("-") {
        if w.len() > 0 {
            let first = w.sub_string(0, 1);
            first.make_upper();
            words.push(first + w.sub_string(1));
        }
    }
    variable::set("app_title", words.reduce(|acc, w| if acc == () { w } else { acc + " " + w }));
}
let owner = variable::get("github_owner");
let lower = owner;
lower.make_lower();
variable::set("image", "ghcr.io/" + lower + "/" + name);
let leaf = variable::get("crate_name");
leaf.replace("_", "");
variable::set("android_app", leaf);
let pkg = "dev.krifmaz." + leaf;
variable::set("android_package", pkg);
// file names (the APK, release artifacts) take the title without spaces: "Demo App" → DemoApp-android-0.1.0.apk
let file = variable::get("app_title");
file.replace(" ", "");
variable::set("app_file", file);
