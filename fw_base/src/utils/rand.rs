use uuid::{Uuid, uuid};

#[inline]
pub fn gen_uuid() -> Uuid {
    Uuid::now_v7()
}

#[cfg(test)]
mod tests {
    use crate::utils::rand::gen_uuid;

    #[test]
    fn test_uuv7() {
        let v = gen_uuid();
        print!("{v}");
    }
}
