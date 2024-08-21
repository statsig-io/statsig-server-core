use jni::JNIEnv;
use jni::objects::JString;

pub fn jstring_to_string(env: &mut JNIEnv, input: JString) -> Option<String> {
    match env.get_string(&input) {
        Ok(s) => Some(s.into()),
        Err(_) => None,
    }
}