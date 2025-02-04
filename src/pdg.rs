use crate::ffi;
use crate::ffi::{
    raw::{PdgEventType, PdgState},
    PDGEventInfo, PDGWorkItemInfo, PDGWorkItemResult,
};
use crate::node::HoudiniNode;
use crate::Result;
use std::fmt::Formatter;
use std::ops::ControlFlow;

/// Represents a single work item.
pub struct PDGWorkItem<'session> {
    pub info: PDGWorkItemInfo,
    pub id: i32,
    pub context_id: i32,
    pub node: &'session HoudiniNode,
}

impl std::fmt::Debug for PDGWorkItem<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PDGWorkItem")
            .field("id", &self.id)
            .field("context", &self.context_id)
            .field("name", &self.info.name(&self.node.session))
            .finish()
    }
}

impl<'session> PDGWorkItem<'session> {
    /// Retrieve the results of work, if the work item has any.
    pub fn get_results(&self) -> Result<Vec<PDGWorkItemResult<'session>>> {
        match self.info.output_file_count() {
            0 => Ok(Vec::new()),
            _ => ffi::get_workitem_result(
                &self.node.session,
                self.node.handle,
                self.id,
                self.info.output_file_count(),
            )
            .map(|results| {
                results
                    .into_iter()
                    .map(|result| PDGWorkItemResult {
                        inner: result,
                        session: (&self.node.session).into(),
                    })
                    .collect()
            }),
        }
    }
}

#[derive(Debug, Clone)]
/// A wrapper for [`HoudiniNode`] with methods for cooking PDG.
pub struct TopNode {
    pub node: HoudiniNode,
}

/// A convenient wrapper for a single event generated by PDG.
pub struct CookStep {
    pub event: PDGEventInfo,
    pub graph_id: i32,
    pub graph_name: i32,
}

// Helper to create a vec of events. No Default impl for it.
fn create_events() -> Vec<ffi::raw::HAPI_PDG_EventInfo> {
    const NUM: usize = 32;
    vec![
        ffi::raw::HAPI_PDG_EventInfo {
            nodeId: -1,
            workItemId: -1,
            dependencyId: -1,
            currentState: -1,
            lastState: -1,
            eventType: -1,
            msgSH: -1,
        };
        NUM
    ]
}

impl TopNode {
    /// Start cooking a TOP node asynchronously.
    /// For each generated event, a user closure will be called with a [`CookStep`] argument.
    ///
    /// The closure returns [`Result<ControlFlow<bool>>`] which is handled like this:
    ///
    /// If its an `Err(_)` - bubble up the error.
    /// If it's [`ControlFlow::Break(bool)`] then the `bool` is either to cancel the cooking
    /// or just break the loop and return.
    /// In case of [`ControlFlow::Continue(_)`] run until completion.
    ///
    /// See the `pdg_cook` example in the `/examples` folder.
    pub fn cook_async<F>(&self, mut func: F) -> Result<()>
    where
        F: FnMut(CookStep) -> Result<ControlFlow<bool>>,
    {
        let session = &self.node.session;
        log::debug!("Start cooking PDG node: {}", self.node.path()?);
        debug_assert!(session.is_valid());
        ffi::cook_pdg(session, self.node.handle, false, false)?;
        let mut events = create_events();
        'main: loop {
            let (graph_ids, graph_names) = ffi::get_pdg_contexts(session)?;
            debug_assert_eq!(graph_ids.len(), graph_names.len());
            for (graph_id, graph_name) in graph_ids.into_iter().zip(graph_names) {
                for event in ffi::get_pdg_events(session, graph_id, &mut events)? {
                    let event = PDGEventInfo { inner: *event };
                    match event.event_type() {
                        PdgEventType::EventCookComplete => break 'main,
                        _ => {
                            match func(CookStep {
                                event,
                                graph_id,
                                graph_name,
                            }) {
                                Err(e) => return Err(e),
                                Ok(ControlFlow::Continue(_)) => {}
                                Ok(ControlFlow::Break(stop_cooking)) => {
                                    if stop_cooking {
                                        // TODO: Should we call this for all graph ids?
                                        ffi::cancel_pdg_cook(session, graph_id)?;
                                    }
                                    break 'main;
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    // FIXME. Observing some weird behaviour. The output files are intermixed with tags
    #[allow(dead_code)]
    #[allow(unreachable_code)]
    fn cook_blocking(&self) -> Result<Vec<PDGWorkItemResult<'_>>> {
        unimplemented!();
        ffi::cook_pdg(&self.node.session, self.node.handle, false, true)?;
        let workitems: Vec<PDGWorkItem> = {
            let context_id = self.get_context_id()?;
            ffi::get_pdg_workitems(&self.node.session, self.node.handle)?
                .into_iter()
                .map(|workitem_id| {
                    Ok(PDGWorkItem {
                        info: PDGWorkItemInfo {
                            inner: ffi::get_workitem_info(
                                &self.node.session,
                                context_id,
                                workitem_id,
                            )?,
                        },
                        id: workitem_id,
                        context_id,
                        node: &self.node,
                    })
                })
                .collect::<Result<Vec<_>>>()?
        };
        let mut all_results = Vec::new();
        for wi in workitems {
            all_results.extend(wi.get_results()?)
        }
        Ok(all_results)
    }

    /// Get the graph(context) id of this node in PDG.
    pub fn get_context_id(&self) -> Result<i32> {
        ffi::get_pdg_context_id(&self.node.session, self.node.handle)
    }

    /// Cancel cooking.
    pub fn cancel_cook(&self) -> Result<()> {
        log::debug!("Cancel PDG cooking {}", self.node.path()?);
        let context = self.get_context_id()?;
        ffi::cancel_pdg_cook(&self.node.session, context)
    }

    /// Dirty the node, forcing the work items to regenerate.
    pub fn dirty_node(&self, clean_results: bool) -> Result<()> {
        log::debug!("Set PDG node dirty {}", self.node.path()?);
        ffi::dirty_pdg_node(&self.node.session, self.node.handle, clean_results)
    }

    /// Which this node current [`PdgState`]
    pub fn get_current_state(&self, context_id: Option<i32>) -> Result<PdgState> {
        let context = match context_id {
            Some(c) => c,
            None => self.get_context_id()?,
        };
        ffi::get_pdg_state(&self.node.session, context)
    }

    /// Get the work item by id and graph(context) id.
    pub fn get_workitem(&self, workitem_id: i32, context_id: i32) -> Result<PDGWorkItem<'_>> {
        ffi::get_workitem_info(&self.node.session, context_id, workitem_id).map(|inner| {
            PDGWorkItem {
                info: PDGWorkItemInfo { inner },
                id: workitem_id,
                context_id,
                node: &self.node,
            }
        })
    }
}
