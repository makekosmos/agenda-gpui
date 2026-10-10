use super::*;

impl Agenda {
    // ==========================================================================
    // TaskBoard parity: options row + list groups.
    // ==========================================================================

    pub(crate) fn board_page(
        &mut self,
        list: SmartList,
        storage: &str,
        fab_hint: bool,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let _ = fab_hint;
        let opts = self.board_opts(storage);

        let mut body = div().flex_1().min_h_0().flex().flex_col().overflow_hidden();

        {
            // Index pipeline + row cache: filter/sort/group reruns only when
            // the list, options or model_rev change — scroll and idle frames
            // reuse the same Rc'd rows.
            let hit = self.row_cache.as_ref().is_some_and(|c| {
                c.list == list
                    && c.group == opts.group
                    && c.sort == opts.sort
                    && c.rev == self.model_rev
            });
            let rows = if hit {
                std::rc::Rc::clone(&self.row_cache.as_ref().unwrap().rows)
            } else {
                // Index pipeline: positions into self.todos, no Todo clones.
                let day = LocalDay::now();
                let items = filter_idx(list, &self.todos, &day);
                let items = sort_idx(&self.todos, items, opts.sort, &day);

                // grouping (same shapes, over indices)
                let groups: Vec<(Option<String>, Vec<usize>)> = match opts.group {
                    GroupKey::None => vec![(None, items)],
                    GroupKey::Project => {
                        let mut g: Vec<(Option<String>, Vec<usize>)> = vec![];
                        let mut no_proj: Vec<usize> = vec![];
                        for p in &self.projects {
                            let bucket: Vec<usize> = items
                                .iter()
                                .copied()
                                .filter(|&i| {
                                    self.todos[i].project_id.as_deref() == Some(p.id.as_str())
                                })
                                .collect();
                            if !bucket.is_empty() {
                                g.push((Some(p.title.clone()), bucket));
                            }
                        }
                        for &i in &items {
                            if self.todos[i].project_id.is_none() {
                                no_proj.push(i);
                            }
                        }
                        if !no_proj.is_empty() {
                            g.insert(0, (Some("Без проекта".to_string()), no_proj));
                        }
                        // unknown ids → "Проект"
                        let orphan: Vec<usize> = items
                            .iter()
                            .copied()
                            .filter(|&i| {
                                self.todos[i]
                                    .project_id
                                    .as_deref()
                                    .is_some_and(|pid| !self.projects.iter().any(|p| p.id == pid))
                            })
                            .collect();
                        if !orphan.is_empty() {
                            g.push((Some("Проект".into()), orphan));
                        }
                        g
                    }
                    GroupKey::Date => {
                        let mut keys: Vec<Option<String>> = vec![];
                        for &i in &items {
                            let k = task_date(&self.todos[i], &day).0;
                            if !keys.contains(&k) {
                                keys.push(k);
                            }
                        }
                        keys.sort_by(|a, b| {
                            a.clone()
                                .unwrap_or_else(|| "9999".into())
                                .cmp(&b.clone().unwrap_or_else(|| "9999".into()))
                        });
                        keys.into_iter()
                            .map(|k| {
                                (
                                    Some(date_group_label(k.as_deref())),
                                    items
                                        .iter()
                                        .copied()
                                        .filter(|&i| task_date(&self.todos[i], &day).0 == k)
                                        .collect::<Vec<usize>>(),
                                )
                            })
                            .collect()
                    }
                };

                let mut flat: Vec<FlatRow> = vec![];
                for (label, gitems) in groups {
                    if gitems.is_empty() {
                        continue;
                    }
                    if let Some(l) = label {
                        flat.push(FlatRow::Header(l, gitems.len()));
                    }
                    flat.extend(gitems.into_iter().map(FlatRow::Task));
                }
                let rows = std::rc::Rc::new(flat);
                self.row_cache = Some(RowCache {
                    list,
                    group: opts.group,
                    sort: opts.sort,
                    rev: self.model_rev,
                    rows: std::rc::Rc::clone(&rows),
                });
                rows
            };
            if rows.is_empty() {
                body = body.child(empty_state_for(list));
            } else {
                body =
                    body.child(self.task_list(format!("list-{storage}"), storage, rows, true, cx));
            }
        }
        body.into_any_element()
    }
}
