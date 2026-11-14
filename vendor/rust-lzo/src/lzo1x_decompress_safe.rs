unsafe extern fn get_unaligned_le16(
    mut p : *const ::std::os::raw::c_void
) -> u16 {
    let mut input : *const u8 = p as (*const u8);
    (*input.offset(0isize) as (i32) | *input.offset(
                                           1isize
                                       ) as (i32) << 8i32) as (u16)
}

#[no_mangle]
pub unsafe extern fn lzo1x_decompress_safe(
    mut in_ : *const u8,
    mut in_len : usize,
    mut out : *mut u8,
    mut out_len : *mut usize
) -> i32 {
    let mut _currentBlock;
    let mut op : *mut u8;
    let mut ip : *const u8;
    let mut t : usize = 0usize;
    let mut next : usize = 0usize;
    let mut state : usize = 0usize;
    let mut m_pos : *const u8;
    let ip_end : *const u8 = in_.offset(in_len as (isize));
    let op_end : *mut u8 = out.offset(*out_len as (isize));
    op = out;
    ip = in_;
    if !(in_len < 3usize) {
        if *ip as (i32) > 17i32 {
            t = (*{
                      let _old = ip;
                      ip = ip.offset(1isize);
                      _old
                  } as (i32) - 17i32) as (usize);
            if t < 4usize {
                next = t;
                _currentBlock = 4;
            } else {
                _currentBlock = 47;
            }
        } else {
            _currentBlock = 7;
        }
        'loop4: loop {
            if _currentBlock == 4 {
                state = next;
                t = next;
                if !(((ip_end as (isize)).wrapping_sub(
                          ip as (isize)
                      ) / ::std::mem::size_of::<u8>(
                          ) as (isize)) as (usize) >= t.wrapping_add(3usize)) {
                    _currentBlock = 56;
                    break;
                }
                if !(((op_end as (isize)).wrapping_sub(
                          op as (isize)
                      ) / ::std::mem::size_of::<u8>() as (isize)) as (usize) >= t) {
                    _currentBlock = 55;
                    break;
                }
                'loop6: loop {
                    if !(t > 0usize) {
                        _currentBlock = 7;
                        break;
                    }
                    *{
                         let _old = op;
                         op = op.offset(1isize);
                         _old
                     } = *{
                              let _old = ip;
                              ip = ip.offset(1isize);
                              _old
                          };
                    t = t.wrapping_sub(1usize);
                }
            } else if _currentBlock == 7 {
                t = *{
                         let _old = ip;
                         ip = ip.offset(1isize);
                         _old
                     } as (usize);
                if t < 16usize {
                    if state == 0usize {
                        if t == 0usize {
                            let mut offset : usize;
                            let mut ip_last : *const u8 = ip;
                            'loop43: loop {
                                if !(*ip as (i32) == 0i32) {
                                    break;
                                }
                                ip = ip.offset(1isize);
                                if !(((ip_end as (isize)).wrapping_sub(
                                          ip as (isize)
                                      ) / ::std::mem::size_of::<u8>(
                                          ) as (isize)) as (usize) >= 1usize) {
                                    _currentBlock = 56;
                                    break 'loop4;
                                }
                            }
                            offset = ((ip as (isize)).wrapping_sub(
                                          ip_last as (isize)
                                      ) / ::std::mem::size_of::<u8>() as (isize)) as (usize);
                            if offset > (!0i32 as (usize)).wrapping_div(255usize).wrapping_sub(
                                            2usize
                                        ) {
                                _currentBlock = 52;
                                break;
                            }
                            offset = (offset << 8i32).wrapping_sub(offset);
                            t = t.wrapping_add(
                                    offset.wrapping_add(15usize).wrapping_add(
                                        *{
                                             let _old = ip;
                                             ip = ip.offset(1isize);
                                             _old
                                         } as (usize)
                                    )
                                );
                        }
                        t = t.wrapping_add(3usize);
                        _currentBlock = 47;
                        continue;
                    } else if state != 4usize {
                        next = t & 3usize;
                        m_pos = op.offset(-1isize) as (*const u8);
                        m_pos = m_pos.offset(-((t >> 2i32) as (isize)));
                        m_pos = m_pos.offset(
                                    -((*{
                                            let _old = ip;
                                            ip = ip.offset(1isize);
                                            _old
                                        } as (i32) << 2i32) as (isize))
                                );
                        if m_pos < out as (*const u8) {
                            _currentBlock = 40;
                            break;
                        }
                        if !(((op_end as (isize)).wrapping_sub(
                                  op as (isize)
                              ) / ::std::mem::size_of::<u8>() as (isize)) as (usize) >= 2usize) {
                            _currentBlock = 55;
                            break;
                        }
                        *op.offset(0isize) = *m_pos.offset(0isize);
                        *op.offset(1isize) = *m_pos.offset(1isize);
                        op = op.offset(2isize);
                        _currentBlock = 4;
                        continue;
                    } else {
                        next = t & 3usize;
                        m_pos = op.offset(-((1i32 + 0x800i32) as (isize))) as (*const u8);
                        m_pos = m_pos.offset(-((t >> 2i32) as (isize)));
                        m_pos = m_pos.offset(
                                    -((*{
                                            let _old = ip;
                                            ip = ip.offset(1isize);
                                            _old
                                        } as (i32) << 2i32) as (isize))
                                );
                        t = 3usize;
                    }
                } else if t >= 64usize {
                    next = t & 3usize;
                    m_pos = op.offset(-1isize) as (*const u8);
                    m_pos = m_pos.offset(-((t >> 2i32 & 7usize) as (isize)));
                    m_pos = m_pos.offset(
                                -((*{
                                        let _old = ip;
                                        ip = ip.offset(1isize);
                                        _old
                                    } as (i32) << 3i32) as (isize))
                            );
                    t = (t >> 5i32).wrapping_sub(1usize).wrapping_add(
                            (3i32 - 1i32) as (usize)
                        );
                } else if t >= 32usize {
                    t = (t & 31usize).wrapping_add((3i32 - 1i32) as (usize));
                    if t == 2usize {
                        let mut offset : usize;
                        let mut ip_last : *const u8 = ip;
                        'loop22: loop {
                            if !(*ip as (i32) == 0i32) {
                                break;
                            }
                            ip = ip.offset(1isize);
                            if !(((ip_end as (isize)).wrapping_sub(
                                      ip as (isize)
                                  ) / ::std::mem::size_of::<u8>(
                                      ) as (isize)) as (usize) >= 1usize) {
                                _currentBlock = 56;
                                break 'loop4;
                            }
                        }
                        offset = ((ip as (isize)).wrapping_sub(
                                      ip_last as (isize)
                                  ) / ::std::mem::size_of::<u8>() as (isize)) as (usize);
                        if offset > (!0i32 as (usize)).wrapping_div(255usize).wrapping_sub(
                                        2usize
                                    ) {
                            _currentBlock = 26;
                            break;
                        }
                        offset = (offset << 8i32).wrapping_sub(offset);
                        t = t.wrapping_add(
                                offset.wrapping_add(31usize).wrapping_add(
                                    *{
                                         let _old = ip;
                                         ip = ip.offset(1isize);
                                         _old
                                     } as (usize)
                                )
                            );
                        if !(((ip_end as (isize)).wrapping_sub(
                                  ip as (isize)
                              ) / ::std::mem::size_of::<u8>() as (isize)) as (usize) >= 2usize) {
                            _currentBlock = 56;
                            break;
                        }
                    }
                    m_pos = op.offset(-1isize) as (*const u8);
                    next = get_unaligned_le16(
                               ip as (*const ::std::os::raw::c_void)
                           ) as (usize);
                    ip = ip.offset(2isize);
                    m_pos = m_pos.offset(-((next >> 2i32) as (isize)));
                    next = next & 3usize;
                } else {
                    m_pos = op as (*const u8);
                    m_pos = m_pos.offset(-(((t & 8usize) << 11i32) as (isize)));
                    t = (t & 7usize).wrapping_add((3i32 - 1i32) as (usize));
                    if t == 2usize {
                        let mut offset : usize;
                        let mut ip_last : *const u8 = ip;
                        'loop12: loop {
                            if !(*ip as (i32) == 0i32) {
                                break;
                            }
                            ip = ip.offset(1isize);
                            if !(((ip_end as (isize)).wrapping_sub(
                                      ip as (isize)
                                  ) / ::std::mem::size_of::<u8>(
                                      ) as (isize)) as (usize) >= 1usize) {
                                _currentBlock = 56;
                                break 'loop4;
                            }
                        }
                        offset = ((ip as (isize)).wrapping_sub(
                                      ip_last as (isize)
                                  ) / ::std::mem::size_of::<u8>() as (isize)) as (usize);
                        if offset > (!0i32 as (usize)).wrapping_div(255usize).wrapping_sub(
                                        2usize
                                    ) {
                            _currentBlock = 18;
                            break;
                        }
                        offset = (offset << 8i32).wrapping_sub(offset);
                        t = t.wrapping_add(
                                offset.wrapping_add(7usize).wrapping_add(
                                    *{
                                         let _old = ip;
                                         ip = ip.offset(1isize);
                                         _old
                                     } as (usize)
                                )
                            );
                        if !(((ip_end as (isize)).wrapping_sub(
                                  ip as (isize)
                              ) / ::std::mem::size_of::<u8>() as (isize)) as (usize) >= 2usize) {
                            _currentBlock = 56;
                            break;
                        }
                    }
                    next = get_unaligned_le16(
                               ip as (*const ::std::os::raw::c_void)
                           ) as (usize);
                    ip = ip.offset(2isize);
                    m_pos = m_pos.offset(-((next >> 2i32) as (isize)));
                    next = next & 3usize;
                    if m_pos == op as (*const u8) {
                        _currentBlock = 17;
                        break;
                    }
                    m_pos = m_pos.offset(-0x4000isize);
                }
                if m_pos < out as (*const u8) {
                    _currentBlock = 40;
                    break;
                }
                let mut oe : *mut u8 = op.offset(t as (isize));
                if !(((op_end as (isize)).wrapping_sub(
                          op as (isize)
                      ) / ::std::mem::size_of::<u8>() as (isize)) as (usize) >= t) {
                    _currentBlock = 55;
                    break;
                }
                *op.offset(0isize) = *m_pos.offset(0isize);
                *op.offset(1isize) = *m_pos.offset(1isize);
                op = op.offset(2isize);
                m_pos = m_pos.offset(2isize);
                'loop35: loop {
                    *{
                         let _old = op;
                         op = op.offset(1isize);
                         _old
                     } = *{
                              let _old = m_pos;
                              m_pos = m_pos.offset(1isize);
                              _old
                          };
                    if !(op < oe) {
                        _currentBlock = 4;
                        break;
                    }
                }
            } else {
                if !(((op_end as (isize)).wrapping_sub(
                          op as (isize)
                      ) / ::std::mem::size_of::<u8>() as (isize)) as (usize) >= t) {
                    _currentBlock = 55;
                    break;
                }
                if !(((ip_end as (isize)).wrapping_sub(
                          ip as (isize)
                      ) / ::std::mem::size_of::<u8>(
                          ) as (isize)) as (usize) >= t.wrapping_add(3usize)) {
                    _currentBlock = 56;
                    break;
                }
                'loop49: loop {
                    *{
                         let _old = op;
                         op = op.offset(1isize);
                         _old
                     } = *{
                              let _old = ip;
                              ip = ip.offset(1isize);
                              _old
                          };
                    if !({
                             t = t.wrapping_sub(1usize);
                             t
                         } > 0usize) {
                        break;
                    }
                }
                state = 4usize;
                _currentBlock = 7;
            }
        }
        if _currentBlock == 56 {
        } else if _currentBlock == 17 {
            *out_len = ((op as (isize)).wrapping_sub(
                            out as (isize)
                        ) / ::std::mem::size_of::<u8>() as (isize)) as (usize);
            return
                if t != 3usize {
                    -1i32
                } else if ip == ip_end {
                    0i32
                } else if ip < ip_end {
                    -8i32
                } else {
                    -4i32
                };
        } else if _currentBlock == 18 {
            return -1i32;
        } else if _currentBlock == 26 {
            return -1i32;
        } else if _currentBlock == 40 {
            *out_len = ((op as (isize)).wrapping_sub(
                            out as (isize)
                        ) / ::std::mem::size_of::<u8>() as (isize)) as (usize);
            return -6i32;
        } else if _currentBlock == 52 {
            return -1i32;
        } else {
            *out_len = ((op as (isize)).wrapping_sub(
                            out as (isize)
                        ) / ::std::mem::size_of::<u8>() as (isize)) as (usize);
            return -5i32;
        }
    }
    *out_len = ((op as (isize)).wrapping_sub(
                    out as (isize)
                ) / ::std::mem::size_of::<u8>() as (isize)) as (usize);
    -4i32
}
