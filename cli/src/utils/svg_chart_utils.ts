import * as d3 from 'd3';
import { csvParse } from 'd3-dsv';
import { JSDOM } from 'jsdom';

export type SvgChartOpts = {
  title: string;
  subtitle: string;
  labels: string[];
  series: number[][];
};

export function generateSvgChart(
  usageDataInput: string,
  eventDataInput: string,
) {
  // timestamp,cpu_percent,mem_usage,mem_limit
  const usageData = csvParse(usageDataInput.trim(), (row) => ({
    timestamp: new Date(Number(row.timestamp)),
    cpu_percent: parseFloat(row.cpu_percent.replace('%', '')),
    mem_usage_mb: parseMemoryToMB(row.mem_usage),
    mem_limit_mb: parseMemoryToMB(row.mem_limit),
  }));

  // timestamp,event
  const eventData = csvParse(eventDataInput.trim(), (row) => ({
    timestamp: new Date(Number(row.timestamp)),
    event: row.event,
  }));

  // Create a fake DOM
  const dom = new JSDOM(`<!DOCTYPE html><body></body>`);
  const body = d3.select(dom.window.document.querySelector('body'));

  // Chart dimensions
  const width = 1200;
  const height = 600;
  const margin = { top: 10, right: 10, bottom: 100, left: 100 };

  // Scales
  const x = d3
    .scaleTime()
    .domain(d3.extent(usageData, (d) => d.timestamp))
    .range([margin.left, width - margin.right]);

  const y = d3
    .scaleLinear()
    .domain([0, d3.max(usageData, (d) => d.mem_usage_mb)])
    .nice()
    .range([height - margin.bottom, margin.top]);

  // Line generator
  const line = d3
    .line<{ timestamp: Date; mem_usage_mb: number }>()
    .x((d) => x(d.timestamp))
    .y((d) => y(d.mem_usage_mb));

  // Create SVG
  const svg = body
    .append('svg')
    .attr('xmlns', 'http://www.w3.org/2000/svg')
    .attr('width', width)
    .attr('height', height);

  // Add white background
  svg
    .append('rect')
    .attr('width', width)
    .attr('height', height)
    .attr('fill', 'white');

  // Add path
  svg
    .append('path')
    .datum(usageData)
    .attr('fill', 'none')
    .attr('stroke', 'steelblue')
    .attr('stroke-width', 2)
    .attr('d', line);

  // Add event divider lines
  svg
    .selectAll('line.event')
    .data(eventData)
    .enter()
    .append('line')
    .attr('class', 'event')
    .attr('x1', (d) => x(d.timestamp))
    .attr('x2', (d) => x(d.timestamp))
    .attr('y1', margin.top)
    .attr('y2', height - margin.bottom)
    .attr('stroke', 'red')
    .attr('stroke-width', 1)
    .attr('opacity', 0.7);

  // Add event labels
  svg
    .selectAll('text.event-label')
    .data(eventData)
    .enter()
    .append('text')
    .attr('class', 'event-label')
    .attr('x', (d) => x(d.timestamp) + 5) // 5px offset from the line
    .attr('y', margin.top + 20) // Position near the top of the chart
    .attr('fill', 'red')
    .attr('font-size', '12px')
    .attr(
      'transform',
      (d) => `rotate(90, ${x(d.timestamp) + 5}, ${margin.top + 20})`,
    )
    .text((d) => d.event);

  // X axis
  svg
    .append('g')
    .attr('transform', `translate(0,${height - margin.bottom})`)
    .call(d3.axisBottom(x).ticks(3).tickFormat(d3.timeFormat('%H:%M:%S')));

  // Y axis
  svg
    .append('g')
    .attr('transform', `translate(${margin.left},0)`)
    .call(d3.axisLeft(y).tickFormat((d) => `${d} MB`));

  return body.html();
}

function parseMemoryToMB(value: string) {
  if (value.endsWith('GiB')) return parseFloat(value) * 1024 * 1.048576; // Convert GiB to MB
  if (value.endsWith('MiB')) return parseFloat(value) * 1.048576; // Convert MiB to MB
  if (value.endsWith('KiB')) return (parseFloat(value) / 1024) * 1.048576; // Convert KiB to MB
  if (value.endsWith('B'))
    return value === '0B' ? 0 : parseFloat(value) / 1000000; // Convert B to MB
  return 0;
}
