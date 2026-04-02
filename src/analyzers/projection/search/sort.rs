use crate::analyzers::ProjectedPageRecord;

pub(super) fn sort_ranked_pages(matches: &mut [(u8, ProjectedPageRecord)]) {
    matches.sort_by(
        |(left_score, left_page): &(u8, ProjectedPageRecord),
         (right_score, right_page): &(u8, ProjectedPageRecord)| {
            right_score
                .cmp(left_score)
                .then_with(|| left_page.title.cmp(&right_page.title))
                .then_with(|| left_page.page_id.cmp(&right_page.page_id))
        },
    );
}
