use super::{Template, TemplateAnchor, TemplateOp};

#[doc(hidden)]
#[allow(missing_docs)]
pub trait TemplateExt {
    fn ops(&self) -> &'static [TemplateOp];

    fn strings(&self) -> &'static [&'static str];

    fn anchors(&self) -> &'static [TemplateAnchor];

    fn dynamic_value_count(&self) -> usize;

    fn root_count(&self) -> usize;

    fn element_meta_at_op(&self, op: usize) -> Option<(&'static str, Option<&'static str>)>;

    fn static_attr_at_op(
        &self,
        op: usize,
    ) -> Option<(&'static str, &'static str, Option<&'static str>)>;

    fn static_text_at_op(&self, op: usize) -> Option<&'static str>;

    fn root_slots(
        &self,
    ) -> impl Iterator<Item = (usize, Option<usize>, Option<&'static TemplateAnchor>)> + '_;

    fn static_children(&self, element_op: usize) -> impl Iterator<Item = usize> + '_;

    fn element_dynamic_anchors(
        &self,
        element_op: usize,
    ) -> impl Iterator<Item = &'static TemplateAnchor> + '_;

    fn static_attrs(
        &self,
        element_op: usize,
    ) -> impl Iterator<Item = (&'static str, &'static str, Option<&'static str>)> + '_;
}

impl TemplateExt for Template {
    fn ops(&self) -> &'static [TemplateOp] {
        Template::ops(self)
    }

    fn strings(&self) -> &'static [&'static str] {
        Template::strings(self)
    }

    fn anchors(&self) -> &'static [TemplateAnchor] {
        Template::anchors(self)
    }

    fn dynamic_value_count(&self) -> usize {
        Template::dynamic_value_count(self)
    }

    fn root_count(&self) -> usize {
        Template::root_count(self)
    }

    fn element_meta_at_op(&self, op: usize) -> Option<(&'static str, Option<&'static str>)> {
        Template::element_meta_at_op(self, op)
    }

    fn static_attr_at_op(
        &self,
        op: usize,
    ) -> Option<(&'static str, &'static str, Option<&'static str>)> {
        Template::static_attr_at_op(self, op)
    }

    fn static_text_at_op(&self, op: usize) -> Option<&'static str> {
        Template::static_text_at_op(self, op)
    }

    fn root_slots(
        &self,
    ) -> impl Iterator<Item = (usize, Option<usize>, Option<&'static TemplateAnchor>)> + '_ {
        Template::root_slots(self)
    }

    fn static_children(&self, element_op: usize) -> impl Iterator<Item = usize> + '_ {
        Template::static_children(self, element_op)
    }

    fn element_dynamic_anchors(
        &self,
        element_op: usize,
    ) -> impl Iterator<Item = &'static TemplateAnchor> + '_ {
        Template::element_dynamic_anchors(self, element_op)
    }

    fn static_attrs(
        &self,
        element_op: usize,
    ) -> impl Iterator<Item = (&'static str, &'static str, Option<&'static str>)> + '_ {
        Template::static_attrs(self, element_op)
    }
}
