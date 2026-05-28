# server/test_virtual_block.py

def group_into_virtual_blocks(raw_lines: list) -> list:
    """
    空间相邻文字块合并合并算法。
    将 OCR 识别出的单个物理框 raw_lines 根据几何相邻性合并为 VirtualBlock。
    """
    if not raw_lines:
        return []

    # 1. 结构化物理框
    blocks = []
    for line in raw_lines:
        box = line[0]
        text = line[1][0].strip()
        confidence = line[1][1]
        
        xs = [pt[0] for pt in box]
        ys = [pt[1] for pt in box]
        x1, y1, x2, y2 = min(xs), min(ys), max(xs), max(ys)
        
        w = x2 - x1
        h = y2 - y1
        cy = y1 + h / 2.0
        
        blocks.append({
            "rect": [x1, y1, x2, y2],
            "text": text,
            "w": w,
            "h": h,
            "cy": cy,
            "confidence": confidence
        })
        
    # 按 Y 坐标排序，方便进行行合并
    blocks.sort(key=lambda b: b["rect"][1])
    
    # 2. 合并同行的空间相邻块
    virtual_blocks = []
    
    while blocks:
        current = blocks.pop(0)
        merged_group = [current]
        
        # 逐个检查剩下的块，判断是否可以与当前合并组内的块在水平方向空间相邻合并
        i = 0
        while i < len(blocks):
            candidate = blocks[i]
            
            # 判断是否可以和合并组中的最后一块合并
            last = merged_group[-1]
            
            avg_h = (last["h"] + candidate["h"]) / 2.0
            
            # 同行判断: 中心 Y 距离小于平均高度的 0.5 倍
            same_line = abs(last["cy"] - candidate["cy"]) <= 0.6 * avg_h
            
            # 水平间距: 间距小于平均高度的 2.0 倍
            gap_x = candidate["rect"][0] - last["rect"][2]
            horizontal_near = 0 <= gap_x <= 2.2 * avg_h
            
            # 高度相近: 比例小于 1.5 倍
            height_similar = (max(last["h"], candidate["h"]) / max(min(last["h"], candidate["h"]), 0.001)) <= 1.5
            
            # 限制合并上限：字符数 <= 80，合并块数 <= 6
            merged_len = sum(len(b["text"]) for b in merged_group) + len(candidate["text"])
            count_ok = len(merged_group) < 6
            
            if same_line and horizontal_near and height_similar and merged_len <= 80 and count_ok:
                merged_group.append(candidate)
                blocks.pop(i)
                # 因为 candidate 被移出了，不需要增加索引 i
            else:
                i += 1
                
        # 聚合合并组，计算 union_bbox 和组合文本
        all_xs = []
        all_ys = []
        texts_to_join = []
        
        for b in merged_group:
            r = b["rect"]
            all_xs.extend([r[0], r[2]])
            all_ys.extend([r[1], r[3]])
            texts_to_join.append(b["text"])
            
        union_x1 = min(all_xs)
        union_y1 = min(all_ys)
        union_x2 = max(all_xs)
        union_y2 = max(all_ys)
        
        # 组合文本中间加一个空格
        union_text = " ".join(texts_to_join)
        avg_height = sum(b["h"] for b in merged_group) / len(merged_group)
        
        virtual_blocks.append({
            "rect": [int(union_x1), int(union_y1), int(union_x2), int(union_y2)],
            "text": union_text,
            "avg_h": avg_height,
            "raw_count": len(merged_group)
        })
        
    return virtual_blocks

def test_virtual_block_merge():
    # 模拟 OCR 返回数据格式
    raw_lines = [
        # Line 1: "Hello" and "World" horizontally adjacent
        [[[10, 10], [50, 10], [50, 25], [10, 25]], ("Hello", 0.99)],
        [[[60, 11], [100, 11], [100, 26], [60, 26]], ("World", 0.98)],
        # Line 2: "Different line" on a lower y-coordinate
        [[[10, 50], [150, 50], [150, 68], [10, 68]], ("Different line", 0.97)]
    ]
    
    vblocks = group_into_virtual_blocks(raw_lines)
    for idx, vb in enumerate(vblocks):
        print(f"VirtualBlock {idx}: Rect={vb['rect']}, Text='{vb['text']}', RawCount={vb['raw_count']}")
        
    assert len(vblocks) == 2
    
    # 验证第一行合并正确
    assert vblocks[0]["text"] == "Hello World"
    assert vblocks[0]["rect"] == [10, 10, 100, 26]
    assert vblocks[0]["raw_count"] == 2
    
    # 验证第二行未受影响
    assert vblocks[1]["text"] == "Different line"
    assert vblocks[1]["rect"] == [10, 50, 150, 68]
    assert vblocks[1]["raw_count"] == 1
    
    print("VirtualBlock merge algorithm unit tests passed!")

if __name__ == "__main__":
    test_virtual_block_merge()
