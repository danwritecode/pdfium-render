//! Defines the [PdfPage] struct, exposing functionality related to a single page in a
//! `PdfPages` collection.

use crate::bindgen::{
    FLATTEN_FAIL, FLATTEN_NOTHINGTODO, FLATTEN_SUCCESS, FLAT_PRINT, FPDF_BOOL, FPDF_PAGE, FS_RECTF,
};
use crate::bindings::PdfiumLibraryBindings;
use crate::bitmap::{PdfBitmap, PdfBitmapFormat, PdfBitmapRotation};
use crate::create_transform_setters;
use crate::document::PdfDocument;
use crate::error::{PdfiumError, PdfiumInternalError};
use crate::font::PdfFont;
use crate::page_boundaries::PdfPageBoundaries;
use crate::page_index_cache::PdfPageIndexCache;
use crate::page_links::PdfPageLinks;
use crate::page_objects::PdfPageObjects;
use crate::page_objects_common::PdfPageObjectsCommon;
use crate::page_size::PdfPagePaperSize;
use crate::page_text::PdfPageText;
use crate::prelude::{PdfMatrix, PdfMatrixValue, PdfPageAnnotations};
use crate::render_config::{PdfRenderConfig, PdfRenderSettings};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::ops::{Add, AddAssign, Div, Mul, Neg, Sub, SubAssign};
use std::os::raw::c_int;

/// The internal coordinate system inside a [PdfDocument] is measured in Points, a
/// device-independent unit equal to 1/72 inches, roughly 0.358 mm. Points are converted to pixels
/// when a [PdfPage] is rendered to a [PdfBitmap].
#[derive(Debug, Copy, Clone, PartialOrd, PartialEq)]
pub struct PdfPoints {
    pub value: f32,
}

impl PdfPoints {
    /// A [PdfPoints] object with the identity value 0.0.
    pub const ZERO: PdfPoints = PdfPoints::zero();

    /// Creates a new [PdfPoints] object with the given value.
    #[inline]
    pub const fn new(value: f32) -> Self {
        Self { value }
    }

    /// Creates a new [PdfPoints] object with the value 0.0.
    ///
    /// Consider using the compile-time constant value [PdfPoints::ZERO]
    /// rather than calling this function directly.
    #[inline]
    pub const fn zero() -> Self {
        Self::new(0.0)
    }

    /// Creates a new [PdfPoints] object from the given measurement in inches.
    #[inline]
    pub fn from_inches(inches: f32) -> Self {
        Self::new(inches * 72.0)
    }

    /// Creates a new [PdfPoints] object from the given measurement in centimeters.
    #[inline]
    pub fn from_cm(cm: f32) -> Self {
        Self::from_inches(cm / 2.54)
    }

    /// Creates a new [PdfPoints] object from the given measurement in millimeters.
    #[inline]
    pub fn from_mm(mm: f32) -> Self {
        Self::from_cm(mm / 10.0)
    }

    /// Converts the value of this [PdfPoints] object to inches.
    #[inline]
    pub fn to_inches(&self) -> f32 {
        self.value / 72.0
    }

    /// Converts the value of this [PdfPoints] object to centimeters.
    #[inline]
    pub fn to_cm(&self) -> f32 {
        self.to_inches() * 2.54
    }

    /// Converts the value of this [PdfPoints] object to millimeters.
    #[inline]
    pub fn to_mm(self) -> f32 {
        self.to_cm() * 10.0
    }
}

impl Add<PdfPoints> for PdfPoints {
    type Output = PdfPoints;

    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        PdfPoints::new(self.value + rhs.value)
    }
}

impl AddAssign<PdfPoints> for PdfPoints {
    #[inline]
    fn add_assign(&mut self, rhs: PdfPoints) {
        self.value += rhs.value;
    }
}

impl Sub<PdfPoints> for PdfPoints {
    type Output = PdfPoints;

    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        PdfPoints::new(self.value - rhs.value)
    }
}

impl SubAssign<PdfPoints> for PdfPoints {
    #[inline]
    fn sub_assign(&mut self, rhs: PdfPoints) {
        self.value -= rhs.value;
    }
}

impl Mul<f32> for PdfPoints {
    type Output = PdfPoints;

    #[inline]
    fn mul(self, rhs: f32) -> Self::Output {
        PdfPoints::new(self.value * rhs)
    }
}

impl Div<f32> for PdfPoints {
    type Output = PdfPoints;

    #[inline]
    fn div(self, rhs: f32) -> Self::Output {
        PdfPoints::new(self.value / rhs)
    }
}

impl Neg for PdfPoints {
    type Output = PdfPoints;

    #[inline]
    fn neg(self) -> Self::Output {
        PdfPoints::new(-self.value)
    }
}

/// A rectangle measured in [PdfPoints].
///
/// The coordinate space of a [PdfPage] has its origin (0,0) at the bottom left of the page,
/// with x values increasing as coordinates move horizontally to the right and
/// y values increasing as coordinates move vertically up.
#[derive(Debug, Copy, Clone)]
pub struct PdfRect {
    pub bottom: PdfPoints,
    pub left: PdfPoints,
    pub top: PdfPoints,
    pub right: PdfPoints,
}

impl PdfRect {
    /// A [PdfRect] object with the identity value (0.0, 0.0, 0.0, 0.0).
    pub const ZERO: PdfRect = PdfRect::zero();

    #[inline]
    pub(crate) fn from_pdfium(rect: FS_RECTF) -> Self {
        Self {
            bottom: PdfPoints::new(rect.bottom),
            left: PdfPoints::new(rect.left),
            top: PdfPoints::new(rect.top),
            right: PdfPoints::new(rect.right),
        }
    }

    #[inline]
    pub(crate) fn from_pdfium_as_result(
        result: FPDF_BOOL,
        rect: FS_RECTF,
        bindings: &dyn PdfiumLibraryBindings,
    ) -> Result<PdfRect, PdfiumError> {
        if result == 0 {
            if let Some(error) = bindings.get_pdfium_last_error() {
                Err(PdfiumError::PdfiumLibraryInternalError(error))
            } else {
                // This would be an unusual situation; a null handle indicating failure,
                // yet Pdfium's error code indicates success.

                Err(PdfiumError::PdfiumLibraryInternalError(
                    PdfiumInternalError::Unknown,
                ))
            }
        } else {
            Ok(PdfRect::from_pdfium(rect))
        }
    }

    /// Creates a new [PdfRect] from the given [PdfPoints] measurements.
    ///
    /// The coordinate space of a [PdfPage] has its origin (0,0) at the bottom left of the page,
    /// with x values increasing as coordinates move horizontally to the right and
    /// y values increasing as coordinates move vertically up.
    #[inline]
    pub const fn new(bottom: PdfPoints, left: PdfPoints, top: PdfPoints, right: PdfPoints) -> Self {
        Self {
            bottom,
            left,
            top,
            right,
        }
    }

    /// Creates a new [PdfRect] from the given raw points values.
    ///
    /// The coordinate space of a [PdfPage] has its origin (0,0) at the bottom left of the page,
    /// with x values increasing as coordinates move horizontally to the right and
    /// y values increasing as coordinates move vertically up.
    #[inline]
    pub const fn new_from_values(bottom: f32, left: f32, top: f32, right: f32) -> Self {
        Self::new(
            PdfPoints::new(bottom),
            PdfPoints::new(left),
            PdfPoints::new(top),
            PdfPoints::new(right),
        )
    }

    /// Creates a new [PdfRect] object with all values set to 0.0.
    ///
    /// Consider using the compile-time constant value [PdfRect::ZERO]
    /// rather than calling this function directly.
    #[inline]
    pub const fn zero() -> Self {
        Self::new_from_values(0.0, 0.0, 0.0, 0.0)
    }

    /// Returns the width of this [PdfRect].
    #[inline]
    pub fn width(&self) -> PdfPoints {
        self.right - self.left
    }

    /// Returns the height of this [PdfRect].
    #[inline]
    pub fn height(&self) -> PdfPoints {
        self.top - self.bottom
    }

    #[inline]
    /// Returns `true` if the given point lies inside this [PdfRect].
    pub fn contains(&self, x: PdfPoints, y: PdfPoints) -> bool {
        self.contains_x(x) && self.contains_y(y)
    }

    #[inline]
    /// Returns `true` if the given horizontal coordinate lies inside this [PdfRect].
    pub fn contains_x(&self, x: PdfPoints) -> bool {
        self.left <= x && self.right >= x
    }

    #[inline]
    /// Returns `true` if the given vertical coordinate lies inside this [PdfRect].
    pub fn contains_y(&self, y: PdfPoints) -> bool {
        self.bottom <= y && self.top >= y
    }

    /// Returns `true` if the bounds of this [PdfRect] lie entirely within the given rectangle.
    #[inline]
    pub fn is_inside(&self, rect: &PdfRect) -> bool {
        self.left >= rect.left
            && self.right <= rect.right
            && self.top <= rect.top
            && self.bottom >= rect.bottom
    }

    /// Returns `true` if the bounds of this [PdfRect] lie at least partially within
    /// the given rectangle.
    #[inline]
    pub fn does_overlap(&self, rect: &PdfRect) -> bool {
        self.left < rect.right
            && self.right > rect.left
            && self.bottom < rect.top
            && self.top > rect.bottom
    }

    #[inline]
    pub(crate) fn as_pdfium(&self) -> FS_RECTF {
        FS_RECTF {
            left: self.left.value,
            top: self.top.value,
            right: self.right.value,
            bottom: self.bottom.value,
        }
    }
}

// We could derive PartialEq automatically, but it's good practice to implement PartialEq
// by hand when implementing Hash.

impl PartialEq for PdfRect {
    fn eq(&self, other: &Self) -> bool {
        self.bottom == other.bottom
            && self.left == other.left
            && self.top == other.top
            && self.right == other.right
    }
}

// The f32 values inside PdfRect will never be NaN or Infinity, so these implementations
// of Eq and Hash are safe.

impl Eq for PdfRect {}

impl Hash for PdfRect {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u32(self.bottom.value.to_bits());
        state.write_u32(self.left.value.to_bits());
        state.write_u32(self.top.value.to_bits());
        state.write_u32(self.right.value.to_bits());
    }
}

/// The orientation of a [PdfPage].
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum PdfPageOrientation {
    Portrait,
    Landscape,
}

impl PdfPageOrientation {
    #[inline]
    pub(crate) fn from_width_and_height(width: PdfPoints, height: PdfPoints) -> Self {
        if width.value > height.value {
            PdfPageOrientation::Landscape
        } else {
            PdfPageOrientation::Portrait
        }
    }
}

/// Content regeneration strategies that instruct `pdfium-render` when, if ever, it should
/// automatically regenerate the content of a [PdfPage].
///
/// Updates to a [PdfPage] are not committed to the underlying [PdfDocument] until the page's
/// content is regenerated. If a page is reloaded or closed without regenerating the page's
/// content, any changes not applied are lost.
///
/// By default, `pdfium-render` will trigger content regeneration on any change to a [PdfPage];
/// this removes the possibility of data loss, and ensures changes can be read back from other
/// data structures as soon as they are made. However, if many changes are made to a page at once,
/// then regenerating the content after every change is inefficient; it is faster to stage
/// all changes first, then regenerate the page's content just once. In this case,
/// changing the content regeneration strategy for a [PdfPage] can improve performance,
/// but you must be careful not to forget to commit your changes before the [PdfPage] moves out of scope.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum PdfPageContentRegenerationStrategy {
    /// `pdfium-render` will call the [PdfPage::regenerate_content()] function on any
    /// change to this [PdfPage]. This is the default setting.
    AutomaticOnEveryChange,

    /// `pdfium-render` will call the [PdfPage::regenerate_content()] function only when
    /// this [PdfPage] is about to move out of scope.
    AutomaticOnDrop,

    /// `pdfium-render` will never call the [PdfPage::regenerate_content()] function.
    /// You must do so manually after staging your changes, or your changes will be lost
    /// when this [PdfPage] moves out of scope.
    Manual,
}

/// A single page in a [PdfDocument].
///
/// In addition to its own intrinsic properties, a [PdfPage] serves as the entry point
/// to all object collections related to a single page in a document. These collections include:
/// * [PdfPage::annotations()], an immutable collection of all the user annotations attached to the [PdfPage].
/// * [PdfPage::annotations_mut()], a mutable collection of all the user annotations attached to the [PdfPage].
/// * [PdfPage::boundaries()], an immutable collection of the boundary boxes relating to the [PdfPage].
/// * [PdfPage::boundaries_mut()], a mutable collection of the boundary boxes relating to the [PdfPage].
/// * [PdfPage::links()], an immutable collection of the links on the [PdfPage].
/// * [PdfPage::links_mut()], a mutable collection of the links on the [PdfPage].
/// * [PdfPage::objects()], an immutable collection of all the displayable objects on the [PdfPage].
/// * [PdfPage::objects_mut()], a mutable collection of all the displayable objects on the [PdfPage].
pub struct PdfPage<'a> {
    handle: FPDF_PAGE,
    label: Option<String>,
    document: &'a PdfDocument<'a>,
    regeneration_strategy: PdfPageContentRegenerationStrategy,
    is_content_regeneration_required: bool,
    annotations: PdfPageAnnotations<'a>,
    boundaries: PdfPageBoundaries<'a>,
    links: PdfPageLinks<'a>,
    objects: PdfPageObjects<'a>,
}

impl<'a> PdfPage<'a> {
    /// The default content regeneration strategy used by `pdfium-render`. This can be overridden
    /// on a page-by-page basis using the [PdfPage::set_content_regeneration_strategy()] function.
    const DEFAULT_CONTENT_REGENERATION_STRATEGY: PdfPageContentRegenerationStrategy =
        PdfPageContentRegenerationStrategy::AutomaticOnEveryChange;

    #[inline]
    pub(crate) fn from_pdfium(
        handle: FPDF_PAGE,
        label: Option<String>,
        document: &'a PdfDocument<'a>,
    ) -> Self {
        let mut result = PdfPage {
            handle,
            label,
            document,
            regeneration_strategy: PdfPageContentRegenerationStrategy::Manual,
            is_content_regeneration_required: false,
            annotations: PdfPageAnnotations::from_pdfium(handle, document),
            boundaries: PdfPageBoundaries::from_pdfium(handle, document.bindings()),
            links: PdfPageLinks::from_pdfium(handle, *document.handle(), document.bindings()),
            objects: PdfPageObjects::from_pdfium(handle, *document.handle(), document.bindings()),
        };

        // Make sure the default content regeneration strategy is applied to child containers.

        result.set_content_regeneration_strategy(Self::DEFAULT_CONTENT_REGENERATION_STRATEGY);

        result
    }

    /// Returns the internal `FPDF_PAGE` handle for this [PdfPage].
    #[inline]
    pub(crate) fn handle(&self) -> &FPDF_PAGE {
        &self.handle
    }

    /// Returns the [PdfDocument] containing this [PdfPage].
    #[inline]
    pub fn document(&self) -> &'a PdfDocument<'a> {
        self.document
    }

    /// Returns the [PdfiumLibraryBindings] used by the [PdfDocument] containing this [PdfPage].
    #[inline]
    pub fn bindings(&self) -> &'a dyn PdfiumLibraryBindings {
        self.document().bindings()
    }

    /// Returns the label assigned to this [PdfPage], if any.
    #[inline]
    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    /// Returns the width of this [PdfPage] in device-independent points.
    /// One point is 1/72 inches, roughly 0.358 mm.
    #[inline]
    pub fn width(&self) -> PdfPoints {
        PdfPoints::new(self.bindings().FPDF_GetPageWidthF(self.handle))
    }

    /// Returns the height of this [PdfPage] in device-independent points.
    /// One point is 1/72 inches, roughly 0.358 mm.
    #[inline]
    pub fn height(&self) -> PdfPoints {
        PdfPoints::new(self.bindings().FPDF_GetPageHeightF(self.handle))
    }

    /// Returns the width and height of this [PdfPage] expressed as a [PdfRect].
    #[inline]
    pub fn page_size(&self) -> PdfRect {
        PdfRect::new(
            PdfPoints::ZERO,
            PdfPoints::ZERO,
            self.height(),
            self.width(),
        )
    }

    /// Returns [PdfPageOrientation::Landscape] if the width of this [PdfPage]
    /// is greater than its height; otherwise returns [PdfPageOrientation::Portrait].
    #[inline]
    pub fn orientation(&self) -> PdfPageOrientation {
        PdfPageOrientation::from_width_and_height(self.width(), self.height())
    }

    /// Returns `true` if this [PdfPage] has orientation [PdfPageOrientation::Portrait].
    #[inline]
    pub fn is_portrait(&self) -> bool {
        self.orientation() == PdfPageOrientation::Portrait
    }

    /// Returns `true` if this [PdfPage] has orientation [PdfPageOrientation::Landscape].
    #[inline]
    pub fn is_landscape(&self) -> bool {
        self.orientation() == PdfPageOrientation::Landscape
    }

    /// Returns any intrinsic rotation encoded into this document indicating a rotation
    /// should be applied to this [PdfPage] during rendering.
    #[inline]
    pub fn rotation(&self) -> Result<PdfBitmapRotation, PdfiumError> {
        PdfBitmapRotation::from_pdfium(self.bindings().FPDFPage_GetRotation(self.handle))
    }

    /// Sets the intrinsic rotation that should be applied to this [PdfPage] during rendering.
    #[inline]
    pub fn set_rotation(&mut self, rotation: PdfBitmapRotation) {
        self.bindings()
            .FPDFPage_SetRotation(self.handle, rotation.as_pdfium());
    }

    /// Returns `true` if any object on the page contains transparency.
    #[inline]
    pub fn has_transparency(&self) -> bool {
        self.bindings()
            .is_true(self.bindings().FPDFPage_HasTransparency(self.handle))
    }

    /// Returns the paper size of this [PdfPage].
    #[inline]
    pub fn paper_size(&self) -> PdfPagePaperSize {
        PdfPagePaperSize::from_points(self.width(), self.height())
    }

    /// Returns `true` if this [PdfPage] contains an embedded thumbnail.
    ///
    /// Embedded thumbnails can be generated as a courtesy by PDF generators to save PDF consumers
    /// the burden of having to render their own thumbnails on the fly. If a thumbnail for this page
    /// was not embedded at the time the document was created, one can easily be rendered using the
    /// standard rendering functions:
    ///
    /// ```
    ///     let thumbnail_desired_pixel_size = 128;
    ///
    ///     let thumbnail = page.render_with_config(
    ///         &PdfRenderConfig::thumbnail(thumbnail_desired_pixel_size)
    ///     )?; // Renders a 128 x 128 thumbnail of the page
    /// ```
    #[inline]
    pub fn has_embedded_thumbnail(&self) -> bool {
        // To determine whether the page includes a thumbnail, we ask Pdfium to return the
        // size of the thumbnail data. A non-zero value indicates a thumbnail exists.

        self.bindings()
            .FPDFPage_GetRawThumbnailData(self.handle, std::ptr::null_mut(), 0)
            > 0
    }

    /// Returns the embedded thumbnail for this [PdfPage], if any.
    ///
    /// Embedded thumbnails can be generated as a courtesy by PDF generators to save PDF consumers
    /// the burden of having to render their own thumbnails on the fly. If a thumbnail for this page
    /// was not embedded at the time the document was created, one can easily be rendered using the
    /// standard rendering functions:
    ///
    /// ```
    ///     let thumbnail_desired_pixel_size = 128;
    ///
    ///     let thumbnail = page.render_with_config(
    ///         &PdfRenderConfig::thumbnail(thumbnail_desired_pixel_size)
    ///     )?; // Renders a 128 x 128 thumbnail of the page
    /// ```
    pub fn embedded_thumbnail(&self) -> Result<PdfBitmap, PdfiumError> {
        let thumbnail_handle = self.bindings().FPDFPage_GetThumbnailAsBitmap(self.handle);

        if thumbnail_handle.is_null() {
            // No thumbnail is available for this page.

            Err(PdfiumError::PageMissingEmbeddedThumbnail)
        } else {
            Ok(PdfBitmap::from_pdfium(thumbnail_handle, self.bindings()))
        }
    }

    /// Returns the collection of text boxes contained within this [PdfPage].
    pub fn text(&self) -> Result<PdfPageText, PdfiumError> {
        if self.regeneration_strategy == PdfPageContentRegenerationStrategy::AutomaticOnEveryChange
            && self.is_content_regeneration_required
        {
            self.regenerate_content_immut()?;
        }

        let text_handle = self.bindings().FPDFText_LoadPage(self.handle);

        if text_handle.is_null() {
            if let Some(error) = self.bindings().get_pdfium_last_error() {
                Err(PdfiumError::PdfiumLibraryInternalError(error))
            } else {
                // This would be an unusual situation; a null handle indicating failure,
                // yet Pdfium's error code indicates success.

                Err(PdfiumError::PdfiumLibraryInternalError(
                    PdfiumInternalError::Unknown,
                ))
            }
        } else {
            Ok(PdfPageText::from_pdfium(text_handle, self, self.bindings()))
        }
    }

    /// Returns an immutable collection of the annotations that have been added to this [PdfPage].
    pub fn annotations(&self) -> &PdfPageAnnotations<'a> {
        if self.regeneration_strategy == PdfPageContentRegenerationStrategy::AutomaticOnEveryChange
            && self.is_content_regeneration_required
        {
            let result = self.regenerate_content_immut();

            debug_assert!(result.is_ok());
        }

        &self.annotations
    }

    /// Returns a mutable collection of the annotations that have been added to this [PdfPage].
    pub fn annotations_mut(&mut self) -> &mut PdfPageAnnotations<'a> {
        // We can't know for sure whether the user will update any annotations,
        // and we can't track what happens in the PdfPageAnnotations instance after we return
        // a mutable reference to it, but if the user is going to the trouble of retrieving
        // a mutable reference it seems best to assume they're intending to update something.

        self.is_content_regeneration_required = self.regeneration_strategy
            != PdfPageContentRegenerationStrategy::AutomaticOnEveryChange;

        self.annotations
            .do_regenerate_page_content_after_each_change(
                self.regeneration_strategy
                    == PdfPageContentRegenerationStrategy::AutomaticOnEveryChange,
            );

        &mut self.annotations
    }

    /// Returns an immutable collection of the bounding boxes defining the extents of this [PdfPage].
    #[inline]
    pub fn boundaries(&self) -> &PdfPageBoundaries<'a> {
        &self.boundaries
    }

    /// Returns a mutable collection of the bounding boxes defining the extents of this [PdfPage].
    #[inline]
    pub fn boundaries_mut(&mut self) -> &mut PdfPageBoundaries<'a> {
        &mut self.boundaries
    }

    /// Returns an immutable collection of the links on this [PdfPage].
    #[inline]
    pub fn links(&self) -> &PdfPageLinks<'a> {
        &self.links
    }

    /// Returns a mutable collection of the links on this [PdfPage].
    #[inline]
    pub fn links_mut(&mut self) -> &mut PdfPageLinks<'a> {
        &mut self.links
    }

    /// Returns an immutable collection of all the page objects on this [PdfPage].
    pub fn objects(&self) -> &PdfPageObjects<'a> {
        if self.regeneration_strategy == PdfPageContentRegenerationStrategy::AutomaticOnEveryChange
            && self.is_content_regeneration_required
        {
            let result = self.regenerate_content_immut();

            debug_assert!(result.is_ok());
        }

        &self.objects
    }

    /// Returns a mutable collection of all the page objects on this [PdfPage].
    pub fn objects_mut(&mut self) -> &mut PdfPageObjects<'a> {
        // We can't know for sure whether the user will update any page objects,
        // and we can't track what happens in the PdfPageObjects instance after we return
        // a mutable reference to it, but if the user is going to the trouble of retrieving
        // a mutable reference it seems best to assume they're intending to update something.

        self.is_content_regeneration_required = self.regeneration_strategy
            != PdfPageContentRegenerationStrategy::AutomaticOnEveryChange;

        self.objects.do_regenerate_page_content_after_each_change(
            self.regeneration_strategy
                == PdfPageContentRegenerationStrategy::AutomaticOnEveryChange,
        );

        &mut self.objects
    }

    /// Returns a list of all the distinct [PdfFont] instances used by the page text objects
    /// on this [PdfPage], if any.
    pub fn fonts(&self) -> Vec<PdfFont> {
        let mut distinct_font_handles = HashMap::new();

        let mut result = Vec::new();

        for object in self.objects().iter() {
            if let Some(object) = object.as_text_object() {
                let font = object.font();

                if !distinct_font_handles.contains_key(font.handle()) {
                    distinct_font_handles.insert(*font.handle(), true);
                    result.push(*font.handle());
                }
            }
        }

        result
            .into_iter()
            .map(|handle| PdfFont::from_pdfium(handle, self.bindings()))
            .collect()
    }

    /// Renders this [PdfPage] into a [PdfBitmap] with the given pixel dimensions and page rotation.
    ///
    /// It is the responsibility of the caller to ensure the given pixel width and height
    /// correctly maintain the page's aspect ratio.
    ///
    /// See also [PdfPage::render_with_config()], which calculates the correct pixel dimensions,
    /// rotation settings, and rendering options to apply from a [PdfRenderConfig] object.
    ///
    /// Each call to `PdfPage::render()` creates a new [PdfBitmap] object and allocates memory
    /// for it. To avoid repeated allocations, create a single [PdfBitmap] object
    /// using [PdfBitmap::empty()] and reuse it across multiple calls to [PdfPage::render_into_bitmap()].
    pub fn render(
        &self,
        width: u16,
        height: u16,
        rotation: Option<PdfBitmapRotation>,
    ) -> Result<PdfBitmap, PdfiumError> {
        let mut bitmap =
            PdfBitmap::empty(width, height, PdfBitmapFormat::default(), self.bindings())?;

        let mut config = PdfRenderConfig::new()
            .set_target_width(width)
            .set_target_height(height);

        if let Some(rotation) = rotation {
            config = config.rotate(rotation, true);
        }

        self.render_into_bitmap_with_config(&mut bitmap, &config)?;

        Ok(bitmap)
    }

    /// Renders this [PdfPage] into a new [PdfBitmap] using pixel dimensions, page rotation settings,
    /// and rendering options configured in the given [PdfRenderConfig].
    ///
    /// Each call to `PdfPage::render_with_config()` creates a new [PdfBitmap] object and
    /// allocates memory for it. To avoid repeated allocations, create a single [PdfBitmap] object
    /// using [PdfBitmap::empty()] and reuse it across multiple calls to
    /// [PdfPage::render_into_bitmap_with_config()].
    pub fn render_with_config(&self, config: &PdfRenderConfig) -> Result<PdfBitmap, PdfiumError> {
        let settings = config.apply_to_page(self);

        let mut bitmap = PdfBitmap::empty(
            settings.width as u16,
            settings.height as u16,
            PdfBitmapFormat::from_pdfium(settings.format as u32)
                .unwrap_or_else(|_| PdfBitmapFormat::default()),
            self.bindings(),
        )?;

        self.render_into_bitmap_with_settings(&mut bitmap, settings)?;

        Ok(bitmap)
    }

    /// Renders this [PdfPage] into the given [PdfBitmap] using the given the given pixel dimensions
    /// and page rotation.
    ///
    /// It is the responsibility of the caller to ensure the given pixel width and height
    /// correctly maintain the page's aspect ratio. The size of the buffer backing the given bitmap
    /// must be sufficiently large to hold the rendered image or an error will be returned.
    ///
    /// See also [PdfPage::render_into_bitmap_with_config()], which calculates the correct pixel dimensions,
    /// rotation settings, and rendering options to apply from a [PdfRenderConfig] object.
    pub fn render_into_bitmap(
        &self,
        bitmap: &mut PdfBitmap,
        width: u16,
        height: u16,
        rotation: Option<PdfBitmapRotation>,
    ) -> Result<(), PdfiumError> {
        let mut config = PdfRenderConfig::new()
            .set_target_width(width)
            .set_target_height(height);

        if let Some(rotation) = rotation {
            config = config.rotate(rotation, true);
        }

        self.render_into_bitmap_with_config(bitmap, &config)
    }

    /// Renders this [PdfPage] into the given [PdfBitmap] using pixel dimensions, page rotation settings,
    /// and rendering options configured in the given [PdfRenderConfig].
    ///
    /// The size of the buffer backing the given bitmap must be sufficiently large to hold the
    /// rendered image or an error will be returned.
    #[inline]
    pub fn render_into_bitmap_with_config(
        &self,
        bitmap: &mut PdfBitmap,
        config: &PdfRenderConfig,
    ) -> Result<(), PdfiumError> {
        self.render_into_bitmap_with_settings(bitmap, config.apply_to_page(self))
    }

    /// Renders this [PdfPage] into the given [PdfBitmap] using the given [PdfRenderSettings].
    /// The size of the buffer backing the given bitmap must be sufficiently large to hold
    /// the rendered image or an error will be returned.
    pub(crate) fn render_into_bitmap_with_settings(
        &self,
        bitmap: &mut PdfBitmap,
        settings: PdfRenderSettings,
    ) -> Result<(), PdfiumError> {
        let bitmap_handle = *bitmap.handle();

        if settings.do_clear_bitmap_before_rendering {
            // Clear the bitmap buffer by setting every pixel to a known color.

            self.bindings().FPDFBitmap_FillRect(
                bitmap_handle,
                0,
                0,
                settings.width,
                settings.height,
                settings.clear_color,
            );

            if let Some(error) = self.bindings().get_pdfium_last_error() {
                return Err(PdfiumError::PdfiumLibraryInternalError(error));
            }
        }

        if settings.do_render_form_data {
            // Render the PDF page into the bitmap buffer, ignoring any custom transformation matrix.
            // (Custom transforms cannot be applied to the rendering of form fields.)

            self.bindings().FPDF_RenderPageBitmap(
                bitmap_handle,
                self.handle,
                0,
                0,
                settings.width,
                settings.height,
                settings.rotate,
                settings.render_flags,
            );

            if let Some(error) = self.bindings().get_pdfium_last_error() {
                return Err(PdfiumError::PdfiumLibraryInternalError(error));
            }

            if let Some(form) = self.document().form() {
                // Render user-supplied form data, if any, as an overlay on top of the page.

                if let Some(form_field_highlight) = settings.form_field_highlight.as_ref() {
                    for (form_field_type, (color, alpha)) in form_field_highlight.iter() {
                        self.bindings().FPDF_SetFormFieldHighlightColor(
                            *form.handle(),
                            *form_field_type,
                            *color,
                        );

                        self.bindings()
                            .FPDF_SetFormFieldHighlightAlpha(*form.handle(), *alpha);
                    }
                }

                self.bindings().FPDF_FFLDraw(
                    *form.handle(),
                    bitmap_handle,
                    self.handle,
                    0,
                    0,
                    settings.width,
                    settings.height,
                    settings.rotate,
                    settings.render_flags,
                );

                if let Some(error) = self.bindings().get_pdfium_last_error() {
                    return Err(PdfiumError::PdfiumLibraryInternalError(error));
                }
            }
        } else {
            // Render the PDF page into the bitmap buffer, applying any custom transformation matrix.

            self.bindings().FPDF_RenderPageBitmapWithMatrix(
                bitmap_handle,
                self.handle,
                &settings.matrix,
                &settings.clipping,
                settings.render_flags,
            );

            if let Some(error) = self.bindings().get_pdfium_last_error() {
                return Err(PdfiumError::PdfiumLibraryInternalError(error));
            }
        }

        Ok(())
    }

    // TODO: AJRC - 29/7/22 - remove deprecated PdfPage::get_bitmap_*() functions in 0.9.0
    // as part of tracking issue https://github.com/ajrcarey/pdfium-render/issues/36
    /// Renders this [PdfPage] into a new [PdfBitmap] using pixel dimensions, rotation settings,
    /// and rendering options configured in the given [PdfRenderConfig].
    #[deprecated(
        since = "0.7.12",
        note = "This function has been renamed to better reflect its purpose. Use the PdfPage::render_with_config() function instead."
    )]
    #[doc(hidden)]
    #[inline]
    pub fn get_bitmap_with_config(
        &self,
        config: &PdfRenderConfig,
    ) -> Result<PdfBitmap, PdfiumError> {
        self.render_with_config(config)
    }

    /// Renders this [PdfPage] into a new [PdfBitmap] with the given pixel dimensions and
    /// rotation setting.
    ///
    /// It is the responsibility of the caller to ensure the given pixel width and height
    /// correctly maintain the page's aspect ratio.
    ///
    /// See also [PdfPage::render_with_config()], which calculates the correct pixel dimensions,
    /// rotation settings, and rendering options to apply from a [PdfRenderConfig] object.
    #[deprecated(
        since = "0.7.12",
        note = "This function has been renamed to better reflect its purpose. Use the PdfPage::render() function instead."
    )]
    #[doc(hidden)]
    pub fn get_bitmap(
        &self,
        width: u16,
        height: u16,
        rotation: Option<PdfBitmapRotation>,
    ) -> Result<PdfBitmap, PdfiumError> {
        self.render(width, height, rotation)
    }

    /// Applies the given transformation, expressed as six values representing the six configurable
    /// elements of a nine-element 3x3 PDF transformation matrix, to the objects on this [PdfPage],
    /// restricting the effects of the transformation to the given clipping rectangle.
    ///
    /// To move, scale, rotate, or skew the objects on this [PdfPage], consider using one or more of
    /// the following functions. Internally they all use [PdfPage::transform()], but are
    /// probably easier to use (and certainly clearer in their intent) in most situations.
    ///
    /// * [PdfPage::translate()]: changes the position of each object on this [PdfPage].
    /// * [PdfPage::scale()]: changes the size of each object on this [PdfPage].
    /// * [PdfPage::rotate_clockwise_degrees()], [PdfPage::rotate_counter_clockwise_degrees()],
    /// [PdfPage::rotate_clockwise_radians()], [PdfPage::rotate_counter_clockwise_radians()]:
    /// rotates each object on this [PdfPage] around its origin.
    /// * [PdfPage::skew_degrees()], [PdfPage::skew_radians()]: skews each object
    /// on this [PdfPage] relative to its axes.
    ///
    /// **The order in which transformations are applied is significant.**
    /// For example, the result of rotating _then_ translating an object may be vastly different
    /// from translating _then_ rotating the same object.
    ///
    /// An overview of PDF transformation matrices can be found in the PDF Reference Manual
    /// version 1.7 on page 204; a detailed description can be founded in section 4.2.3 on page 207.
    #[inline]
    #[allow(clippy::too_many_arguments)]
    pub fn transform_with_clip(
        &mut self,
        a: PdfMatrixValue,
        b: PdfMatrixValue,
        c: PdfMatrixValue,
        d: PdfMatrixValue,
        e: PdfMatrixValue,
        f: PdfMatrixValue,
        clip: PdfRect,
    ) -> Result<(), PdfiumError> {
        self.set_matrix_with_clip(PdfMatrix::new(a, b, c, d, e, f), clip)
    }

    /// Applies the values in the given [PdfMatrix] to this [PdfPage], restricting the effects
    /// of the transformation matrix to the given clipping rectangle.
    pub fn set_matrix_with_clip(
        &mut self,
        matrix: PdfMatrix,
        clip: PdfRect,
    ) -> Result<(), PdfiumError> {
        self.bindings().FPDFPage_TransFormWithClip(
            self.handle,
            &matrix.as_pdfium(),
            &clip.as_pdfium(),
        );

        match self.bindings().get_pdfium_last_error() {
            Some(err) => Err(PdfiumError::PdfiumLibraryInternalError(err)),
            None => Ok(()),
        }
    }

    create_transform_setters!(
        &mut Self,
        Result<(), PdfiumError>,
        "each object on this [PdfPage]",
        "each object on this [PdfPage].",
        "each object on this [PdfPage],"
    );

    // The transform_impl() function required by the create_transform_setters!() macro
    // is provided by the PdfPageObjectPrivate trait.

    #[inline]
    fn transform_impl(
        &mut self,
        a: PdfMatrixValue,
        b: PdfMatrixValue,
        c: PdfMatrixValue,
        d: PdfMatrixValue,
        e: PdfMatrixValue,
        f: PdfMatrixValue,
    ) -> Result<(), PdfiumError> {
        self.transform_with_clip(
            a,
            b,
            c,
            d,
            e,
            f,
            PdfRect::new(
                PdfPoints::ZERO,
                PdfPoints::ZERO,
                self.height(),
                self.width(),
            ),
        )
    }

    /// Flattens all annotations and form fields on this [PdfPage] into the page contents.
    pub fn flatten(&mut self) -> Result<(), PdfiumError> {
        // TODO: AJRC - 28/5/22 - consider allowing the caller to set the FLAT_NORMALDISPLAY or FLAT_PRINT flag.
        let flag = FLAT_PRINT;

        match self.bindings().FPDFPage_Flatten(self.handle, flag as c_int) as u32 {
            FLATTEN_SUCCESS => {
                self.is_content_regeneration_required = true;

                self.regenerate_content()
            }
            FLATTEN_NOTHINGTODO => Ok(()),
            FLATTEN_FAIL => Err(PdfiumError::PageFlattenFailure),
            _ => Err(PdfiumError::PageFlattenFailure),
        }
    }

    /// Deletes this [PdfPage] from its containing `PdfPages` collection, consuming this [PdfPage].
    pub fn delete(self) -> Result<(), PdfiumError> {
        let index = PdfPageIndexCache::get_index_for_page(*self.document.handle(), self.handle)
            .ok_or(PdfiumError::SourcePageIndexNotInCache)?;

        self.bindings()
            .FPDFPage_Delete(*self.document().handle(), index as c_int);

        if let Some(error) = self.bindings().get_pdfium_last_error() {
            Err(PdfiumError::PdfiumLibraryInternalError(error))
        } else {
            PdfPageIndexCache::delete_pages_at_index(*self.document.handle(), index, 1);

            Ok(())
        }
    }

    /// Returns the strategy used by `pdfium-render` to regenerate the content of a [PdfPage].
    ///
    /// Updates to a [PdfPage] are not committed to the underlying [PdfDocument] until the page's
    /// content is regenerated. If a page is reloaded or closed without regenerating the page's
    /// content, all uncommitted changes will be lost.
    ///
    /// By default, `pdfium-render` will trigger content regeneration on any change to a [PdfPage];
    /// this removes the possibility of data loss, and ensures changes can be read back from other
    /// data structures as soon as they are made. However, if many changes are made to a page at once,
    /// then regenerating the content after every change is inefficient; it is faster to stage
    /// all changes first, then regenerate the page's content just once. In this case,
    /// changing the content regeneration strategy for a [PdfPage] can improve performance,
    /// but you must be careful not to forget to commit your changes before closing
    /// or reloading the page.
    #[inline]
    pub fn content_regeneration_strategy(&self) -> PdfPageContentRegenerationStrategy {
        self.regeneration_strategy
    }

    /// Sets the strategy used by `pdfium-render` to regenerate the content of a [PdfPage].
    ///
    /// Updates to a [PdfPage] are not committed to the underlying [PdfDocument] until the page's
    /// content is regenerated. If a page is reloaded or closed without regenerating the page's
    /// content, all uncommitted changes will be lost.
    ///
    /// By default, `pdfium-render` will trigger content regeneration on any change to a [PdfPage];
    /// this removes the possibility of data loss, and ensures changes can be read back from other
    /// data structures as soon as they are made. However, if many changes are made to a page at once,
    /// then regenerating the content after every change is inefficient; it is faster to stage
    /// all changes first, then regenerate the page's content just once. In this case,
    /// changing the content regeneration strategy for a [PdfPage] can improve performance,
    /// but you must be careful not to forget to commit your changes before closing
    /// or reloading the page.
    #[inline]
    pub fn set_content_regeneration_strategy(
        &mut self,
        strategy: PdfPageContentRegenerationStrategy,
    ) {
        self.regeneration_strategy = strategy;
        self.objects.do_regenerate_page_content_after_each_change(
            self.regeneration_strategy
                == PdfPageContentRegenerationStrategy::AutomaticOnEveryChange,
        );
    }

    /// Commits any staged but unsaved changes to this [PdfPage] to the underlying [PdfDocument].
    ///
    /// Updates to a [PdfPage] are not committed to the underlying [PdfDocument] until the page's
    /// content is regenerated. If a page is reloaded or closed without regenerating the page's
    /// content, all uncommitted changes will be lost.
    ///
    /// By default, `pdfium-render` will trigger content regeneration on any change to a [PdfPage];
    /// this removes the possibility of data loss, and ensures changes can be read back from other
    /// data structures as soon as they are made. However, if many changes are made to a page at once,
    /// then regenerating the content after every change is inefficient; it is faster to stage
    /// all changes first, then regenerate the page's content just once. In this case,
    /// changing the content regeneration strategy for a [PdfPage] can improve performance,
    /// but you must be careful not to forget to commit your changes before closing
    /// or reloading the page.
    #[inline]
    pub fn regenerate_content(&mut self) -> Result<(), PdfiumError> {
        // This is a publicly-visible wrapper for the private regenerate_content_immut() function.
        // It is only available to callers who hold a mutable reference to the page.

        self.regenerate_content_immut()
    }

    /// Commits any staged but unsaved changes to this [PdfPage] to the underlying [PdfDocument].
    pub(crate) fn regenerate_content_immut(&self) -> Result<(), PdfiumError> {
        Self::regenerate_content_immut_for_handle(self.handle, self.bindings())
    }

    /// Commits any staged but unsaved changes to the page identified by the given internal
    /// `FPDF_PAGE` handle to the underlying [PdfDocument] containing that page.
    pub(crate) fn regenerate_content_immut_for_handle(
        page: FPDF_PAGE,
        bindings: &dyn PdfiumLibraryBindings,
    ) -> Result<(), PdfiumError> {
        if bindings.is_true(bindings.FPDFPage_GenerateContent(page)) {
            Ok(())
        } else {
            Err(PdfiumError::PdfiumLibraryInternalError(
                bindings
                    .get_pdfium_last_error()
                    .unwrap_or(PdfiumInternalError::Unknown),
            ))
        }
    }
}

impl<'a> Drop for PdfPage<'a> {
    /// Closes this [PdfPage], releasing held memory.
    #[inline]
    fn drop(&mut self) {
        if self.regeneration_strategy != PdfPageContentRegenerationStrategy::Manual
            && self.is_content_regeneration_required
        {
            // Regenerate page content now if necessary, before the PdfPage moves out of scope.

            let result = self.regenerate_content();

            debug_assert!(result.is_ok());
        }

        self.bindings().FPDF_ClosePage(self.handle);

        PdfPageIndexCache::remove_index_for_page(*self.document.handle(), self.handle);
    }
}

#[cfg(test)]
mod test {
    use crate::prelude::*;
    use crate::utils::test::test_bind_to_pdfium;

    #[test]
    fn test_pdf_rect_is_inside() {
        assert!(PdfRect::new_from_values(3.0, 3.0, 9.0, 9.0)
            .is_inside(&PdfRect::new_from_values(2.0, 2.0, 10.0, 10.0)));

        assert!(!PdfRect::new_from_values(2.0, 2.0, 10.0, 10.0)
            .is_inside(&PdfRect::new_from_values(3.0, 3.0, 9.0, 9.0)));

        assert!(!PdfRect::new_from_values(2.0, 2.0, 7.0, 7.0)
            .is_inside(&PdfRect::new_from_values(5.0, 4.0, 10.0, 10.0)));

        assert!(!PdfRect::new_from_values(2.0, 2.0, 7.0, 7.0)
            .is_inside(&PdfRect::new_from_values(8.0, 4.0, 10.0, 10.0)));

        assert!(!PdfRect::new_from_values(2.0, 2.0, 7.0, 7.0)
            .is_inside(&PdfRect::new_from_values(5.0, 8.0, 10.0, 10.0)));
    }

    #[test]
    fn test_pdf_rect_does_overlap() {
        assert!(PdfRect::new_from_values(2.0, 2.0, 7.0, 7.0)
            .does_overlap(&PdfRect::new_from_values(5.0, 4.0, 10.0, 10.0)));

        assert!(!PdfRect::new_from_values(2.0, 2.0, 7.0, 7.0)
            .does_overlap(&PdfRect::new_from_values(8.0, 4.0, 10.0, 10.0)));

        assert!(!PdfRect::new_from_values(2.0, 2.0, 7.0, 7.0)
            .does_overlap(&PdfRect::new_from_values(5.0, 8.0, 10.0, 10.0)));
    }

    #[test]
    fn test_page_rendering_reusing_bitmap() -> Result<(), PdfiumError> {
        // Renders each page in the given test PDF file to a separate JPEG file
        // by re-using the same bitmap buffer for each render.

        let pdfium = test_bind_to_pdfium();

        let document = pdfium.load_pdf_from_file("./test/export-test.pdf", None)?;

        let render_config = PdfRenderConfig::new()
            .set_target_width(2000)
            .set_maximum_height(2000)
            .rotate_if_landscape(PdfBitmapRotation::Degrees90, true);

        let mut bitmap =
            PdfBitmap::empty(2500, 2500, PdfBitmapFormat::default(), pdfium.bindings())?;

        for (index, page) in document.pages().iter().enumerate() {
            page.render_into_bitmap_with_config(&mut bitmap, &render_config)?; // Re-uses the same bitmap for rendering each page.

            bitmap
                .as_image()
                .as_rgba8()
                .ok_or(PdfiumError::ImageError)?
                .save_with_format(format!("test-page-{}.jpg", index), image::ImageFormat::Jpeg)
                .map_err(|_| PdfiumError::ImageError)?;
        }

        Ok(())
    }
}
