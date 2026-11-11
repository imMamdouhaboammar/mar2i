//! Ministry of Hajj and Umrah Standard Media Profiles & Presets.
//!
//! Provides institutional, pre-configured transcoding and archival configurations
//! compliant with Ministry archival policies.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MinistryProfile {
    pub id: String,
    pub name_ar: String,
    pub name_en: String,
    pub description_ar: String,
    pub description_en: String,
    pub mode: String,
    pub codec: String,
    pub container: String,
    pub crf: Option<u8>,
    pub hardware_accelerated: bool,
    pub verification: String, // "framemd5_lossless" | "ssim_perceptual"
}

pub fn standard_profiles() -> Vec<MinistryProfile> {
    vec![
        MinistryProfile {
            id: "master_archive".into(),
            name_ar: "الأرشفة الوثائقية غير المنقوصة".into(),
            name_en: "Master Archival (FFV1 Lossless)".into(),
            description_ar: "حفظ فوتوغرافي ومرئي للأصول الأصلية دون فقد رياضي، مع فحص تطابق البصمات الإطارية framemd5".into(),
            description_en: "Mathematical lossless archiving using FFV1 in MKV container with frame-md5 verification".into(),
            mode: "archive".into(),
            codec: "ffv1".into(),
            container: "mkv".into(),
            crf: Some(0),
            hardware_accelerated: false,
            verification: "framemd5_lossless".into(),
        },
        MinistryProfile {
            id: "broadcast_hd".into(),
            name_ar: "البث التلفزيوني عالي الدقة (4K/1080p)".into(),
            name_en: "Broadcasting Master (HEVC 10-bit)".into(),
            description_ar: "ترميز عالي الكفاءة مخصص للتوزيع التلفزيوني والبث الرسمي بجودة بصرية فائقة ومساحة تخزين مثالية".into(),
            description_en: "High-efficiency HEVC 10-bit encoding optimized for broadcast television and official releases".into(),
            mode: "publish".into(),
            codec: "hevc".into(),
            container: "mp4".into(),
            crf: Some(22),
            hardware_accelerated: true,
            verification: "ssim_perceptual".into(),
        },
        MinistryProfile {
            id: "press_universal".into(),
            name_ar: "النشر الإعلامي وشبكات التواصل".into(),
            name_en: "Universal Press & Social (H.264)".into(),
            description_ar: "أقصى درجات التوافق مع كافة المنصات والأجهزة المحمولة والمتصفحات، ملائم للمراكز الصحفية والإعلامية".into(),
            description_en: "Universal H.264 profile compatible with all mobile devices, web players, and press kits".into(),
            mode: "publish".into(),
            codec: "h264".into(),
            container: "mp4".into(),
            crf: Some(24),
            hardware_accelerated: true,
            verification: "ssim_perceptual".into(),
        },
        MinistryProfile {
            id: "field_mobile".into(),
            name_ar: "التداول الميداني السريع (شبكات المشاعر)".into(),
            name_en: "Field Mobile Delivery (AV1)".into(),
            description_ar: "أعلى معدل توفير في الحجم بواسطة AV1 لضمان سرعة الإرسال والمشاركة عبر شبكات الهواتف الميدانية في منى وعرفات".into(),
            description_en: "High-compression AV1 profile designed for low-bandwidth cellular field transfer across pilgrimage sites".into(),
            mode: "publish".into(),
            codec: "av1".into(),
            container: "mp4".into(),
            crf: Some(28),
            hardware_accelerated: false,
            verification: "ssim_perceptual".into(),
        },
        MinistryProfile {
            id: "review_proxy".into(),
            name_ar: "نسخة المعاينة والتدقيق الداخلي".into(),
            name_en: "Internal Review Proxy".into(),
            description_ar: "نسخة خفيفة وسريعة التوليد للمراجعة الإدارية واختيار المشاهد قبل اتخاذ قرار النشر أو الأرشفة".into(),
            description_en: "Lightweight proxy copy for rapid administrative preview and review before final encoding".into(),
            mode: "publish".into(),
            codec: "h264".into(),
            container: "mp4".into(),
            crf: Some(30),
            hardware_accelerated: true,
            verification: "ssim_perceptual".into(),
        },
    ]
}

pub fn get_profile_by_id(id: &str) -> Option<MinistryProfile> {
    standard_profiles().into_iter().find(|p| p.id == id)
}
