use crate::utils::element;

pub(crate) struct StatisticsView;

pub(crate) enum StatisticType {
    TotalPipelines(usize),
    RunningTasks(usize),
    FailedTasks(usize),
}

impl StatisticsView {
    pub(crate) fn update(stats: &StatisticType) {
        use StatisticType::*;

        let (selector, count) = match stats {
            TotalPipelines(count) => ("#pipelines-count", count),
            RunningTasks(count) => ("#running-count", count),
            FailedTasks(count) => ("#failed-count", count),
        };

        if let Some(el) = element(selector) {
            el.set_inner_html(&count.to_string());
        }
    }
}
