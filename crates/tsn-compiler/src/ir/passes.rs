use super::{IrBlock, IrFunction, IrTerminator};
use rustc_hash::{FxHashMap, FxHashSet};

pub fn optimize_function(mut func: IrFunction) -> IrFunction {
    func.children = func.children.into_iter().map(optimize_function).collect();
    remove_unreachable_blocks(&mut func);
    simplify_trivial_jumps(&mut func);
    remove_unreachable_blocks(&mut func);
    func
}

fn remove_unreachable_blocks(func: &mut IrFunction) {
    if func.blocks.is_empty() {
        return;
    }

    let mut reachable = FxHashSet::default();
    let mut stack = vec![0usize];
    while let Some(id) = stack.pop() {
        if !reachable.insert(id) {
            continue;
        }
        if let Some(block) = func.blocks.get(id) {
            for succ in &block.successors {
                if *succ < func.blocks.len() {
                    stack.push(*succ);
                }
            }
        }
    }

    if reachable.len() == func.blocks.len() {
        return;
    }

    let mut remap = FxHashMap::default();
    let mut new_blocks = Vec::with_capacity(reachable.len());
    for old_id in 0..func.blocks.len() {
        if !reachable.contains(&old_id) {
            continue;
        }
        remap.insert(old_id, new_blocks.len());
        let mut block = func.blocks[old_id].clone();
        block.id = new_blocks.len();
        block.predecessors.clear();
        block.successors.clear();
        new_blocks.push(block);
    }

    for block in &mut new_blocks {
        let old_succ = compute_successors_from_term(&block.terminator, &func.blocks);
        let mut succ = Vec::with_capacity(old_succ.len());
        for old in old_succ {
            if let Some(new_id) = remap.get(&old) {
                succ.push(*new_id);
            }
        }
        succ.sort_unstable();
        succ.dedup();
        block.successors = succ;
    }

    for idx in 0..new_blocks.len() {
        let succs = new_blocks[idx].successors.clone();
        for succ in succs {
            if let Some(target) = new_blocks.get_mut(succ) {
                target.predecessors.push(idx);
            }
        }
    }

    func.blocks = new_blocks;
}

fn simplify_trivial_jumps(func: &mut IrFunction) {
    if func.blocks.is_empty() {
        return;
    }

    let mut ip_to_block = FxHashMap::default();
    for block in &func.blocks {
        ip_to_block.insert(block.start_ip, block.id);
    }

    for idx in 0..func.blocks.len() {
        if matches!(
            func.blocks[idx].terminator,
            IrTerminator::Return | IrTerminator::Throw
        ) {
            continue;
        }

        let new_term = match &func.blocks[idx].terminator {
            IrTerminator::Jump { target_ip } => {
                let final_ip = chase_jump_target(*target_ip, &ip_to_block, func, 16);
                IrTerminator::Jump {
                    target_ip: final_ip,
                }
            }
            IrTerminator::Loop { target_ip } => {
                let final_ip = chase_jump_target(*target_ip, &ip_to_block, func, 16);
                IrTerminator::Loop {
                    target_ip: final_ip,
                }
            }
            IrTerminator::Branch {
                opcode,
                then_ip,
                else_ip,
            } => {
                let then_final = chase_jump_target(*then_ip, &ip_to_block, func, 16);
                let else_final = chase_jump_target(*else_ip, &ip_to_block, func, 16);
                if then_final == else_final {
                    IrTerminator::Jump {
                        target_ip: then_final,
                    }
                } else {
                    IrTerminator::Branch {
                        opcode: *opcode,
                        then_ip: then_final,
                        else_ip: else_final,
                    }
                }
            }
            IrTerminator::Fallthrough { next_ip } => {
                if let Some(ip) = next_ip {
                    let final_ip = chase_jump_target(*ip, &ip_to_block, func, 16);
                    IrTerminator::Fallthrough {
                        next_ip: Some(final_ip),
                    }
                } else {
                    IrTerminator::Fallthrough { next_ip: None }
                }
            }
            IrTerminator::Return | IrTerminator::Throw => continue,
        };
        func.blocks[idx].terminator = new_term;
    }

    for idx in 0..func.blocks.len() {
        let succ = compute_successors_from_term(&func.blocks[idx].terminator, &func.blocks);
        func.blocks[idx].successors = succ;
        func.blocks[idx].predecessors.clear();
    }
    for idx in 0..func.blocks.len() {
        let succs = func.blocks[idx].successors.clone();
        for succ in succs {
            if let Some(target) = func.blocks.get_mut(succ) {
                target.predecessors.push(idx);
            }
        }
    }
}

fn chase_jump_target(
    start_ip: usize,
    ip_to_block: &FxHashMap<usize, usize>,
    func: &IrFunction,
    max_hops: usize,
) -> usize {
    let mut current_ip = start_ip;
    let mut hops = 0usize;
    let mut seen = FxHashSet::default();

    while hops < max_hops && seen.insert(current_ip) {
        let Some(block_id) = ip_to_block.get(&current_ip).copied() else {
            break;
        };
        let Some(block) = func.blocks.get(block_id) else {
            break;
        };
        if !block.instrs.is_empty() {
            break;
        }
        match block.terminator {
            IrTerminator::Jump { target_ip } | IrTerminator::Loop { target_ip } => {
                current_ip = target_ip;
                hops += 1;
            }
            IrTerminator::Fallthrough { next_ip } => {
                if let Some(next) = next_ip {
                    current_ip = next;
                    hops += 1;
                } else {
                    break;
                }
            }
            IrTerminator::Branch { .. } | IrTerminator::Return | IrTerminator::Throw => break,
        }
    }

    current_ip
}

fn compute_successors_from_term(term: &IrTerminator, blocks: &[IrBlock]) -> Vec<usize> {
    let mut out = Vec::new();
    match term {
        IrTerminator::Jump { target_ip } | IrTerminator::Loop { target_ip } => {
            if let Some(id) = block_id_by_start_ip(*target_ip, blocks) {
                out.push(id);
            }
        }
        IrTerminator::Branch {
            then_ip, else_ip, ..
        } => {
            if let Some(id) = block_id_by_start_ip(*then_ip, blocks) {
                out.push(id);
            }
            if let Some(id) = block_id_by_start_ip(*else_ip, blocks) {
                out.push(id);
            }
            out.sort_unstable();
            out.dedup();
        }
        IrTerminator::Fallthrough { next_ip } => {
            if let Some(ip) = next_ip {
                if let Some(id) = block_id_by_start_ip(*ip, blocks) {
                    out.push(id);
                }
            }
        }
        IrTerminator::Return | IrTerminator::Throw => {}
    }
    out
}

fn block_id_by_start_ip(start_ip: usize, blocks: &[IrBlock]) -> Option<usize> {
    blocks
        .iter()
        .find_map(|b| (b.start_ip == start_ip).then_some(b.id))
}
