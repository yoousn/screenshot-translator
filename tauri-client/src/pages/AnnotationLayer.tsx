import React, { useState, useRef } from 'react';
import { Stage, Layer, Line, Rect, Arrow } from 'react-konva';
import { Button, Space } from 'antd';
import { EditOutlined, BorderOutlined, ArrowUpOutlined, CloseOutlined, CheckOutlined } from '@ant-design/icons';

interface AnnotationLayerProps {
  width: number;
  height: number;
  onSave: (dataUrl: string) => void;
  onCancel: () => void;
  baseImageObj: HTMLImageElement | null;
}

export const AnnotationLayer: React.FC<AnnotationLayerProps> = ({ width, height, onSave, onCancel, baseImageObj }) => {
  const [tool, setTool] = useState<'pen' | 'rect' | 'arrow' | 'none'>('pen');
  const [lines, setLines] = useState<any[]>([]);
  const [rects, setRects] = useState<any[]>([]);
  const [arrows, setArrows] = useState<any[]>([]);
  const isDrawing = useRef(false);
  const stageRef = useRef<any>(null);

  const handleMouseDown = (e: any) => {
    if (tool === 'none') return;
    isDrawing.current = true;
    const pos = e.target.getStage().getPointerPosition();
    if (tool === 'pen') {
      setLines([...lines, { tool, points: [pos.x, pos.y] }]);
    } else if (tool === 'rect') {
      setRects([...rects, { x: pos.x, y: pos.y, width: 0, height: 0 }]);
    } else if (tool === 'arrow') {
      setArrows([...arrows, { points: [pos.x, pos.y, pos.x, pos.y] }]);
    }
  };

  const handleMouseMove = (e: any) => {
    if (!isDrawing.current || tool === 'none') return;
    const stage = e.target.getStage();
    const point = stage.getPointerPosition();
    
    if (tool === 'pen') {
      let lastLine = lines[lines.length - 1];
      lastLine.points = lastLine.points.concat([point.x, point.y]);
      lines.splice(lines.length - 1, 1, lastLine);
      setLines(lines.concat());
    } else if (tool === 'rect') {
      let lastRect = rects[rects.length - 1];
      lastRect.width = point.x - lastRect.x;
      lastRect.height = point.y - lastRect.y;
      rects.splice(rects.length - 1, 1, lastRect);
      setRects(rects.concat());
    } else if (tool === 'arrow') {
      let lastArrow = arrows[arrows.length - 1];
      lastArrow.points = [lastArrow.points[0], lastArrow.points[1], point.x, point.y];
      arrows.splice(arrows.length - 1, 1, lastArrow);
      setArrows(arrows.concat());
    }
  };

  const handleMouseUp = () => {
    isDrawing.current = false;
  };

  const handleSave = () => {
    if (stageRef.current) {
      const uri = stageRef.current.toDataURL({ pixelRatio: 2 });
      // Remove data:image/png;base64,
      onSave(uri.replace(/^data:image\/png;base64,/, ''));
    }
  };

  return (
    <div style={{ position: 'relative', width, height }}>
      <Stage
        width={width}
        height={height}
        onMouseDown={handleMouseDown}
        onMousemove={handleMouseMove}
        onMouseup={handleMouseUp}
        ref={stageRef}
      >
        <Layer>
          {baseImageObj && (
             <React.Fragment>
                {/* For Konva we need to draw native Image using Konva.Image component, 
                    since we only have HTMLImageElement, let's use a custom Image */}
             </React.Fragment>
          )}
        </Layer>
        <Layer>
          {lines.map((line, i) => (
            <Line
              key={i}
              points={line.points}
              stroke="#ff4d4f"
              strokeWidth={3}
              tension={0.5}
              lineCap="round"
              lineJoin="round"
            />
          ))}
          {rects.map((rect, i) => (
            <Rect
              key={i}
              x={rect.x}
              y={rect.y}
              width={rect.width}
              height={rect.height}
              stroke="#ff4d4f"
              strokeWidth={3}
            />
          ))}
          {arrows.map((arrow, i) => (
            <Arrow
              key={i}
              points={arrow.points}
              stroke="#ff4d4f"
              strokeWidth={3}
              pointerLength={10}
              pointerWidth={10}
              fill="#ff4d4f"
            />
          ))}
        </Layer>
      </Stage>
      <div style={{ position: 'absolute', bottom: -40, left: 0, background: '#fff', padding: '4px 8px', borderRadius: '4px', boxShadow: '0 2px 8px rgba(0,0,0,0.15)', display: 'flex', gap: '8px' }}>
        <Button size="small" type={tool === 'pen' ? 'primary' : 'default'} icon={<EditOutlined />} onClick={() => setTool('pen')} />
        <Button size="small" type={tool === 'rect' ? 'primary' : 'default'} icon={<BorderOutlined />} onClick={() => setTool('rect')} />
        <Button size="small" type={tool === 'arrow' ? 'primary' : 'default'} icon={<ArrowUpOutlined />} onClick={() => setTool('arrow')} />
        <Button size="small" icon={<CloseOutlined />} onClick={onCancel} />
        <Button size="small" type="primary" icon={<CheckOutlined />} onClick={handleSave} />
      </div>
    </div>
  );
};
