import copy
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch, MagicMock
import runner


def circuit(gates,n=2):return dict(schema_version=1,qubits=n,gates=gates)
def g(name,q=0,target=None):return dict(name=name,qubit=q,**({'target':target} if target is not None else {}))

class BridgeTests(unittest.TestCase):
    def test_provider_simulators(self):
        cases=[(circuit([]),{'00':1}), (circuit([g('x')]),{'01':1}),
               (circuit([g('x',1)]),{'10':1}),
               (circuit([g('h'),g('cnot',0,1)]),{'00':.5,'11':.5}),
               (circuit([g('h'),g('cnot',0,1),g('cnot',1,2)],3),{'000':.5,'111':.5}),
               (circuit([g('h'),g('z'),g('h')]),{'01':1}),
               (circuit([g('h'),g('s'),g('s'),g('h')]),{'01':1}),
               (circuit([g('h'),g('t'),g('h')]),{'00':.8535533905932737,'01':.1464466094067262}),
               (circuit([g('x',1),g('cnot',1,0)]),{'11':1})]
        for provider in ['aws','google']:
            for data,expected in cases:
                with self.subTest(provider=provider,data=data):
                    result=runner.execute(data,provider,4096)
                    self.assertEqual(set(result['counts']),set(expected))
                    for bit,p in expected.items():self.assertLess(abs(result['counts'][bit]/4096-p),.04)
                    self.assertEqual(result['status'],'completed')

    def test_reject_untrusted_input(self):
        bad=[{}, circuit([g('eval')]),circuit([g('x',True)]),circuit([g('cnot',0,0)]),circuit([],9),circuit([g('x')]*65)]
        for data in bad:
            with self.assertRaises(ValueError):runner.validate(data)
        for shots in [0,10001,True]:
            with self.assertRaises(ValueError):runner.execute(circuit([]),'aws',shots)
        with self.assertRaises(ValueError):runner.execute(circuit([]),'google',submit=True)

    def test_aws_receipt_and_no_duplicate_path(self):
        with tempfile.TemporaryDirectory() as tmp,patch('braket.aws.AwsDevice') as factory:
            task=factory.return_value.run.return_value;task.id='task-id'
            task.result.return_value.measurement_counts={'10':4}
            dest=str(Path(tmp)/'receipt.json')
            r=runner.execute(circuit([g('x')]),'aws',4,True,'arn:aws:braket:region:device/test',receipt=dest)
            self.assertEqual(r['counts'],{'01':4})
            self.assertEqual(json.loads(Path(dest).read_text())['job_id'],'task-id')
            with self.assertRaises(FileExistsError):runner.execute(circuit([]),'aws',4,True,'arn:aws:braket:region:device/test',receipt=dest)
            self.assertEqual(factory.return_value.run.call_count,1)

    def test_google_device_validation_before_submission(self):
        with tempfile.TemporaryDirectory() as tmp,patch('cirq_google.Engine') as factory:
            engine=factory.return_value
            engine.get_processor.return_value.get_device.return_value.validate_circuit.side_effect=ValueError('Unsupported topology')
            with self.assertRaises(ValueError):runner.execute(circuit([g('cnot',0,1)]),'google',4,True,project='test',processor='test',mapping=[[0,0],[0,2]],receipt=str(Path(tmp)/'r.json'))
            engine.run_sweep.assert_not_called()

    def test_job_id_survives_poll_failure(self):
        with tempfile.TemporaryDirectory() as tmp,patch('braket.aws.AwsDevice') as factory:
            task=factory.return_value.run.return_value;task.id='recoverable-task-id';task.result.side_effect=TimeoutError()
            dest=str(Path(tmp)/'r.json')
            with self.assertRaises(TimeoutError):runner.execute(circuit([]),'aws',4,True,'arn:aws:braket:region:device/test',receipt=dest)
            r=json.loads(Path(dest).read_text());self.assertEqual(r['job_id'],'recoverable-task-id');self.assertEqual(r['status'],'failed_or_unconfirmed')

if __name__=='__main__':unittest.main()
