// Fills in the names derived from the project name, so a new app needs only its name:
//   app_title        "Filebrary" for filebrary (asked; empty = the project name with each word capitalised)
//   android_package  dev.krifmaz.<crate_name>, the Android applicationId and Kotlin package
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
variable::set("android_package", "dev.krifmaz." + variable::get("crate_name"));
