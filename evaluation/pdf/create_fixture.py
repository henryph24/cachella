"""Recreate the synthetic three-page citation fixture (requires reportlab)."""
from pathlib import Path
from reportlab.pdfgen import canvas

output = Path(__file__).parent / 'fixtures' / 'project-brief.pdf'
output.parent.mkdir(parents=True, exist_ok=True)
pdf = canvas.Canvas(str(output), pagesize=(595, 842), invariant=1)
pdf.setTitle('Synthetic Cachella PDF citation fixture')
pdf.setFont('Helvetica-Bold', 18)
pdf.drawString(48, 784, 'Orion launch decision')
pdf.setFont('Helvetica', 12)
pdf.drawString(48, 745, 'The Orion launch is scheduled for Friday.')
pdf.drawString(48, 723, 'Priya owns the permission fix required before launch.')
pdf.drawString(48, 48, '1')
pdf.showPage()
pdf.showPage()
pdf.setFont('Helvetica-Bold', 18)
pdf.drawString(48, 784, 'Orion infrastructure budget')
pdf.setFont('Helvetica', 12)
pdf.drawString(48, 745, 'The Orion infrastructure budget is nine dollars per month.')
pdf.drawString(48, 723, 'Local inference runs on employee computers.')
pdf.drawString(48, 48, '3')
pdf.save()
print(f'Created {output}: 3 pages; page 2 intentionally blank')
