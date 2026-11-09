import 'package:flutter_test/flutter_test.dart';
import 'package:max_drop/main.dart';

void main() {
  testWidgets('MAX Notch App renders', (WidgetTester tester) async {
    await tester.pumpWidget(const MaxNotchApp());
    expect(find.byType(MaxNotchShell), findsOneWidget);
  });
}
